use base64::Engine;
use reqwest::Client;
use serde::Serialize;
use zeroize::Zeroize;

use crate::domain::storage::StorageError;
use crate::infra::runtime_secret::SecretMaterial;

pub(crate) struct SensitiveString(Vec<u8>);

impl SensitiveString {
    pub(crate) fn new(value: String) -> Self {
        Self(value.into_bytes())
    }

    pub(crate) fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("sensitive runtime strings are created from UTF-8")
    }
}

impl Clone for SensitiveString {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Drop for SensitiveString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub(crate) struct StorageCredentials {
    username: String,
    password: SecretMaterial,
}

impl StorageCredentials {
    pub(crate) fn new(username: &str, password: SecretMaterial) -> Result<Self, StorageError> {
        if username.is_empty() {
            return Err(StorageError::ConnectionFailed(
                "storage runtime identity username is empty".to_string(),
            ));
        }
        Ok(Self {
            username: username.to_string(),
            password,
        })
    }

    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn password(&self) -> &str {
        self.password.expose()
    }

    pub(crate) fn basic_auth(&self) -> SensitiveString {
        let mut credentials = format!("{}:{}", self.username, self.password()).into_bytes();
        let auth = SensitiveString::new(format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(&credentials)
        ));
        credentials.zeroize();
        auth
    }
}

impl std::fmt::Debug for StorageCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StorageCredentials")
            .field("username", &self.username)
            .field("credential_present", &true)
            .finish()
    }
}

#[derive(Serialize)]
struct SigninRequest<'a> {
    #[serde(rename = "NS")]
    namespace: &'a str,
    #[serde(rename = "DB")]
    database: &'a str,
    user: &'a str,
    pass: &'a str,
}

pub(crate) async fn signin(
    client: &Client,
    url: &str,
    namespace: &str,
    database: &str,
    credentials: &StorageCredentials,
) -> Result<SensitiveString, StorageError> {
    let body = SigninRequest {
        namespace,
        database,
        user: credentials.username(),
        pass: credentials.password(),
    };
    let response = client
        .post(format!("{url}/signin"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| StorageError::ConnectionFailed(format!("/signin failed: {error}")))?;

    let status = response.status();
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|error| StorageError::ConnectionFailed(format!("/signin parse: {error}")))?;
    if !status.is_success() {
        return Err(StorageError::ConnectionFailed(format!(
            "/signin returned {status}: {}",
            json.get("details")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
        )));
    }
    let token = json
        .get("token")
        .and_then(|value| value.as_str())
        .ok_or_else(|| StorageError::ConnectionFailed("/signin: no token in response".into()))?;
    Ok(SensitiveString::new(format!("Bearer {token}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_debug_is_presence_only() {
        let credentials = StorageCredentials::new(
            "cynic-runtime",
            SecretMaterial::new("runtime-password-value".to_string()),
        )
        .unwrap();

        let debug = format!("{credentials:?}");
        assert!(!debug.contains("runtime-password-value"));
        assert!(debug.contains("credential_present"));
    }
}
