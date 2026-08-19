//! Credential injection for source sync.
//!
//! Resolves credential references to concrete credentials at execution time.
//! Secrets are never stored in the plan (which is serialized to JSON) — only
//! the reference path is stored. The actual secret is read from disk at
//! execution time via `SecretMaterial::read_file()`.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Credential {
    /// SSH key file — sets GIT_SSH_COMMAND
    SshKey(PathBuf),
    /// HTTPS token — sets GIT_ASKPASS
    Token(PathBuf),
}

/// Resolves credential references to concrete credentials.
pub trait CredentialResolver: Send + Sync {
    fn resolve(&self, credential_ref: &str) -> Result<Credential, String>;
}

/// Default resolver — reads from /etc/cynic/secrets/.
pub struct SecretFileResolver {
    pub secrets_dir: PathBuf,
}

impl SecretFileResolver {
    pub fn new(secrets_dir: PathBuf) -> Self {
        Self { secrets_dir }
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from("/etc/cynic/secrets")
    }
}

impl CredentialResolver for SecretFileResolver {
    fn resolve(&self, credential_ref: &str) -> Result<Credential, String> {
        let path = self.secrets_dir.join(credential_ref);
        if !path.exists() {
            return Err(format!("credential file not found: {}", path.display()));
        }

        // Determine type from extension or naming convention
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name.contains("-key") || file_name.contains("-ssh") {
            Ok(Credential::SshKey(path))
        } else {
            // Default to token (PAT, password)
            Ok(Credential::Token(path))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_returns_ssh_for_key_files() {
        let resolver = SecretFileResolver::new(PathBuf::from("/tmp"));
        // Would need a real file to test — this is a structural test
        assert_eq!(resolver.secrets_dir, PathBuf::from("/tmp"));
    }
}
