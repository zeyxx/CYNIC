//! Transport and HTTP/CLI API functions for Forgejo tokens.

use std::fs::OpenOptions;
use std::io::Write as IoWrite;
use std::os::unix::fs::OpenOptionsExt;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

use crate::forgejo_token::{
    ForgejoTokenAuthState, ForgejoTokenMetadata,
};


pub(super) fn run_forgejo_tokens_add(
    spec: &ForgejoTokenSpec,
) -> Result<(String, ForgejoTokenMetadata), String> {
    let mut cmd = Command::new("forgejo");
    cmd.args([
        "tokensAdd",
        &spec.principal,
        "--scopes",
        &spec.scopes.join(","),
    ])
    .arg("--format")
    .arg("json");

    if let Some(expiry) = spec.expiry_seconds {
        cmd.args(["--expiry", &format!("{}s", expiry)]);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("forgejo tokensAdd spawn failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "forgejo tokensAdd failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Forgejo tokensAdd --format json emits {"sha1":"...","token":"..."}
    let value: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("forgejo JSON parse failed: {e}"))?;
    let token = value
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "forgejo tokensAdd did not return a token field".to_string())?;
    let metadata = extract_token_metadata(&value)?;
    Ok((token.to_string(), metadata))
}

pub(super) fn forgejo_token_is_revoked(principal: &str, token_sha1: &str) -> Result<bool, String> {
    let output = Command::new("forgejo")
        .args(["tokens", principal, "--format", "json"])
        .output()
        .map_err(|e| format!("forgejo token list spawn failed: {e}"))?;
    if !output.status.success() {
        return Err("forgejo token list failed during revocation read-back".to_string());
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("forgejo token list JSON parse failed: {e}"))?;
    let entries = value
        .as_array()
        .ok_or_else(|| "forgejo token list returned a non-array".to_string())?;
    Ok(!entries
        .iter()
        .any(|entry| entry.get("sha1").and_then(|v| v.as_str()) == Some(token_sha1)))
}

pub(super) fn forgejo_token_revoke(principal: &str, sha1: &str) -> Result<(), String> {
    let output = Command::new("forgejo")
        .args(["tokensDelete", principal])
        .arg(sha1)
        .output()
        .map_err(|e| format!("forgejo tokensDelete spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "forgejo tokensDelete failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

pub(super) fn forgejo_api_validate_token(token_path: &str, api_base: &str) -> Result<bool, String> {
    if token_path != forgejo_token_path().display().to_string() {
        return Err("token API probe requires the canonical token path".to_string());
    }
    let mut token = std::fs::read_to_string(forgejo_token_path())
        .map_err(|e| format!("read token file: {e}"))?;
    let token_value = token.trim().to_string();
    token.zeroize();
    let client = http_client()?;
    let result = client
        .get(format_api_url(api_base, "/user"))
        .header("Authorization", format!("token {token_value}"))
        .send()
        .map(|resp| resp.status().is_success())
        .map_err(|e| format!("API probe request failed: {e}"));
    drop(token_value);
    result
}

pub(super) fn forgejo_http_create_token(
    api_base: &str,
    principal: &str,
    scopes: &str,
    authorization_token: Option<&str>,
) -> Result<serde_json::Value, String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let body = serde_json::json!({ "name": principal, "scopes": scopes });
    let resp = client
        .post(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
        .json(&body)
        .send()
        .map_err(|e| format!("token creation request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token creation HTTP {}", resp.status()));
    }
    resp.json()
        .map_err(|e| format!("token creation JSON parse failed: {e}"))
}

pub(super) fn forgejo_http_token_is_revoked(
    api_base: &str,
    principal: &str,
    token_sha1: &str,
    authorization_token: Option<&str>,
) -> Result<bool, String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let resp = client
        .get(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
        .send()
        .map_err(|e| format!("token revocation read-back request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token revocation read-back HTTP {}", resp.status()));
    }
    let tokens: serde_json::Value = resp
        .json()
        .map_err(|e| format!("token revocation read-back JSON parse failed: {e}"))?;
    let entries = tokens
        .as_array()
        .ok_or_else(|| "token revocation read-back returned a non-array".to_string())?;
    Ok(!entries
        .iter()
        .any(|entry| entry.get("sha1").and_then(|v| v.as_str()) == Some(token_sha1)))
}

pub(super) fn forgejo_http_revoke_token(
    api_base: &str,
    principal: &str,
    sha1: &str,
    authorization_token: Option<&str>,
) -> Result<(), String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let resp = client
        .delete(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens/{sha1}"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
        .send()
        .map_err(|e| format!("token revoke request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token revoke HTTP {}", resp.status()));
    }
    Ok(())
}

fn extract_token_from_api_response(resp: &serde_json::Value) -> Result<String, String> {
    resp.get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "API response missing token field".to_string())
}

fn extract_api_token_material(
    resp: &serde_json::Value,
) -> Result<(String, ForgejoTokenMetadata), String> {
    let mut token = extract_token_from_api_response(resp)?;
    let metadata = match extract_token_metadata(resp) {
        Ok(metadata) => metadata,
        Err(error) => {
            token.zeroize();
            return Err(error);
        }
    };
    Ok((token, metadata))
}

fn validate_token_identity(sha1: &str) -> Result<(), String> {
    if sha1.len() != 40 {
        return Err("Forgejo token identity must be exactly 40 hexadecimal characters".to_string());
    }
    if !sha1.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Forgejo token identity must be hexadecimal".to_string());
    }
    Ok(())
}

fn extract_token_metadata(resp: &serde_json::Value) -> Result<ForgejoTokenMetadata, String> {
    Ok(ForgejoTokenMetadata {
        token_id: resp
            .get("id")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| "token response missing exact id".to_string())?,
        name: resp
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "token response missing exact name".to_string())?
            .to_string(),
        sha1: resp
            .get("sha1")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "token response missing exact sha1".to_string())?
            .to_string(),
    })
}

fn required_authorization(token: Option<&str>) -> Result<&str, String> {
    token
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Forgejo HTTP mutation requires Authorization token".to_string())
}

fn format_api_url(api_base: &str, path: &str) -> String {
    let base = api_base.trim_end_matches('/');
    let base = base.strip_suffix("/api/v1").unwrap_or(base);
    format!("{base}/api/v1/{}", path.trim_start_matches('/'))
}

pub(super) fn write_protected_secret(target: &str, token: &str, mode: u32) -> Result<(), String> {
    if target != forgejo_token_path().display().to_string() {
        return Err(format!(
            "secret target must be canonical path {}",
            forgejo_token_path().display()
        ));
    }
    let path = PathBuf::from(target);
    let parent = path
        .parent()
        .ok_or_else(|| "secret target has no parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("secret directory creation failed: {e}"))?;
    if parent
        .canonicalize()
        .map_err(|e| format!("secret directory canonicalization failed: {e}"))?
        != forgejo_token_directory()
    {
        return Err("secret directory must not resolve through a symlink".to_string());
    }
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        if metadata.file_type().is_symlink() {
            return Err("secret target symlink rejected".to_string());
        }
        if !metadata.file_type().is_file() {
            return Err("secret target is not a regular file".to_string());
        }
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_nanos();
    let temporary = parent.join(format!(".forgejo-token.{}.{}", std::process::id(), nonce));
    let mut file = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .custom_flags(0o400000)
        .open(&temporary)
    {
        Ok(file) => file,
        Err(e) => {
            let _ = std::fs::remove_file(&temporary);
            return Err(format!("secret temporary file write failed: {e}"));
        }
    };
    if let Err(e) = file.write_all(token.as_bytes()) {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret write_all failed: {e}"));
    }
    if let Err(e) = file.sync_all() {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret sync failed: {e}"));
    }
    drop(file);
    if let Err(e) = std::fs::rename(&temporary, &path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret atomic rename failed: {e}"));
    }
    Ok(())
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("http client build failed: {e}"))
}

#[cfg(test)]
