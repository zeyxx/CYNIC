//! Live upstream release probing for Python (PyPI), Node (npm), and GitHub releases.
//!
//! Fact-checks latest published versions on demand without static hardcoding.

use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamVersionInfo {
    pub dep_id: String,
    pub latest_version: String,
    pub source: UpstreamSourceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamSourceKind {
    PyPI,
    Npm,
    GitHub,
}

#[derive(Deserialize)]
struct PyPIReleaseInfo {
    info: PyPIInfoData,
}

#[derive(Deserialize)]
struct PyPIInfoData {
    version: String,
}

#[derive(Deserialize)]
struct NpmPackageInfo {
    version: String,
}

#[derive(Deserialize)]
struct GitHubReleaseInfo {
    tag_name: String,
}

/// Query PyPI JSON API for latest package version.
pub fn probe_pypi_latest_version(distribution: &str) -> Result<String, String> {
    let url = format!("https://pypi.org/pypi/{distribution}/json");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?;

    let resp: PyPIReleaseInfo = client
        .get(&url)
        .header("User-Agent", "cynic-tech-watch/1.0")
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    Ok(resp.info.version)
}

/// Query GitHub Releases API for latest tag_name.
pub fn probe_github_latest_release(owner_repo: &str) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{owner_repo}/releases/latest");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?;

    let resp: GitHubReleaseInfo = client
        .get(&url)
        .header("User-Agent", "cynic-tech-watch/1.0")
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let tag = resp.tag_name.trim_start_matches('v').to_string();
    Ok(tag)
}

/// Query npm registry for latest package version.
pub fn probe_npm_latest_version(package_name: &str) -> Result<String, String> {
    let url = format!("https://registry.npmjs.org/{package_name}/latest");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?;

    let resp: NpmPackageInfo = client
        .get(&url)
        .header("User-Agent", "cynic-tech-watch/1.0")
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    Ok(resp.version)
}

/// Dynamic upstream version probe dispatcher.
pub fn probe_upstream_version(
    dep_id: &str,
    python_dist: Option<&str>,
    github_repo: Option<&str>,
    npm_pkg: Option<&str>,
) -> Option<UpstreamVersionInfo> {
    if let Some(repo) = github_repo {
        if let Ok(version) = probe_github_latest_release(repo) {
            return Some(UpstreamVersionInfo {
                dep_id: dep_id.to_string(),
                latest_version: version,
                source: UpstreamSourceKind::GitHub,
            });
        }
    }
    if let Some(dist) = python_dist {
        if let Ok(version) = probe_pypi_latest_version(dist) {
            return Some(UpstreamVersionInfo {
                dep_id: dep_id.to_string(),
                latest_version: version,
                source: UpstreamSourceKind::PyPI,
            });
        }
    }
    if let Some(pkg) = npm_pkg {
        if let Ok(version) = probe_npm_latest_version(pkg) {
            return Some(UpstreamVersionInfo {
                dep_id: dep_id.to_string(),
                latest_version: version,
                source: UpstreamSourceKind::Npm,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pypi_probe_parses_json_structure() {
        let json = r#"{"info":{"version":"0.19.0"}}"#;
        let parsed: PyPIReleaseInfo = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.info.version, "0.19.0");
    }

    #[test]
    fn github_release_parses_tag() {
        let json = r#"{"tag_name":"v2026.8.3"}"#;
        let parsed: GitHubReleaseInfo = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.tag_name.trim_start_matches('v'), "2026.8.3");
    }
}
