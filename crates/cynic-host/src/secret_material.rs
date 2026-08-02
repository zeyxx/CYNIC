use std::io::Read;
use zeroize::Zeroize;

const MAX_SECRET_BYTES: u64 = 64 * 1024;
const RUNTIME_SECRET_RANDOM_BYTES: usize = 32;

pub(crate) struct SecretMaterial(Vec<u8>);

impl SecretMaterial {
    pub(crate) fn read_human_admin_stdin() -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_SECRET_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("human admin stdin unavailable: {error}"))?;
        if bytes.len() as u64 > MAX_SECRET_BYTES {
            bytes.zeroize();
            return Err("human admin secret exceeds the bounded size".to_string());
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
        }
        Self::from_bytes(bytes)
    }

    pub(crate) fn generate_runtime_identity() -> Result<Self, String> {
        let mut random = [0u8; RUNTIME_SECRET_RANDOM_BYTES];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut source| source.read_exact(&mut random))
            .map_err(|error| format!("runtime identity entropy unavailable: {error}"))?;
        let encoded = hex::encode(random);
        random.zeroize();
        Self::from_bytes(encoded.into_bytes())
    }

    pub(crate) fn expose_bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn expose_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("secret material is validated as UTF-8")
    }

    #[cfg(test)]
    pub(crate) fn fixture(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.is_empty() {
            return Err("secret material is empty".to_string());
        }
        if std::str::from_utf8(&bytes).is_err() {
            bytes.zeroize();
            return Err("secret material is not valid UTF-8".to_string());
        }
        Ok(Self(bytes))
    }
}

impl std::fmt::Debug for SecretMaterial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretMaterial([REDACTED])")
    }
}

impl Drop for SecretMaterial {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_projects_material() {
        let secret = SecretMaterial::fixture("machine-enrollment-secret");
        assert!(!format!("{secret:?}").contains("machine-enrollment-secret"));
    }

    #[test]
    fn generated_runtime_identity_has_bounded_high_entropy_encoding() {
        let secret = SecretMaterial::generate_runtime_identity().unwrap();
        assert_eq!(secret.expose_str().len(), RUNTIME_SECRET_RANDOM_BYTES * 2);
        assert!(
            secret
                .expose_str()
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
    }
}
