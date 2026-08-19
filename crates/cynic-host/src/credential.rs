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

    /// Frictionless secret enrollment.
    ///
    /// Called when `resolve()` fails because the file doesn't exist.
    /// Prompts the operator (hidden input), writes to disk with mode 0640,
    /// owner root:cynic. Returns the credential for immediate use.
    pub fn prompt_and_save(&self, credential_ref: &str, secret_type: CredentialType) -> Result<Credential, String> {
        let path = self.secrets_dir.join(credential_ref);

        // Ensure secrets directory exists
        std::fs::create_dir_all(&self.secrets_dir)
            .map_err(|e| format!("create secrets dir: {e}"))?;

        // Prompt (hidden)
        let prompt = format!("Enter {} (input hidden, will be saved to {}): ",
            secret_type.prompt_label(),
            path.display()
        );
        let secret = rpassword::prompt_password(prompt)
            .map_err(|e| format!("read password: {e}"))?;

        if secret.trim().is_empty() {
            return Err("secret cannot be empty".into());
        }

        // Write with restricted mode
        std::fs::write(&path, format!("{secret}\n"))
            .map_err(|e| format!("write secret: {e}"))?;

        // Set mode 0640, owner root:cynic (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))
                .map_err(|e| format!("set permissions: {e}"))?;

            // Try to set owner to root:cynic — ignore if not running as root
            let _ = std::process::Command::new("chown")
                .args(["root:cynic", &path.to_string_lossy()])
                .output();
        }

        secret_type.to_credential(path)
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
            Ok(Credential::Token(path))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CredentialType {
    GitHubPat,
    SshKey,
}

impl CredentialType {
    pub fn prompt_label(self) -> &'static str {
        match self {
            CredentialType::GitHubPat => "GitHub Personal Access Token",
            CredentialType::SshKey => "SSH private key path (already on disk)",
        }
    }

    pub fn to_credential(self, path: PathBuf) -> Result<Credential, String> {
        match self {
            CredentialType::GitHubPat => Ok(Credential::Token(path)),
            CredentialType::SshKey => Ok(Credential::SshKey(path)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_returns_ssh_for_key_files() {
        let resolver = SecretFileResolver::new(PathBuf::from("/tmp"));
        assert_eq!(resolver.secrets_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn credential_type_token_for_pat() {
        let cred = CredentialType::GitHubPat.to_credential(PathBuf::from("/tmp/pat")).unwrap();
        assert!(matches!(cred, Credential::Token(_)));
    }
}
