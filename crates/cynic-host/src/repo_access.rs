use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const REPO_ACCESS_STATUS_SCHEMA: &str = "cynic.runtime_principals.repo_access.status.v1";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepoAccessEntry {
    pub path: String,
    pub owner: Option<String>,
    pub added_by: Option<String>,
    pub discovered: bool,
    pub added_at: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoAccessReadiness {
    pub path: String,
    pub owner: Option<String>,
    pub source: String,
    pub discovered: bool,
    pub added_by: Option<String>,
    pub added_at: String,
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoAccessStatusReport {
    pub schema: &'static str,
    pub mutates: bool,
    pub profile_path: String,
    pub profile_exists: bool,
    pub canonical_project_root: Option<String>,
    pub known_repos: Vec<String>,
    pub entries: Vec<RepoAccessReadiness>,
}

#[derive(Debug, Deserialize)]
struct RepoAccessProfileDocument {
    #[allow(dead_code)]
    schema: Option<String>,
    repositories: Option<Vec<RepoAccessEntryRaw>>,
}

#[derive(Debug, Deserialize)]
struct RepoAccessEntryRaw {
    path: Option<String>,
    owner: Option<String>,
    added_by: Option<String>,
    discovered: Option<bool>,
    added_at: Option<String>,
    source: Option<String>,
}

pub fn current_repo_access_status() -> RepoAccessStatusReport {
    let profile_path = current_repo_access_profile_path();
    let canonical_project_root = current_project_root();
    repo_access_status(&profile_path, canonical_project_root.as_deref())
}

pub fn current_repo_access_profile_path() -> PathBuf {
    env::var("CYNIC_REPO_ACCESS_PROFILE")
        .map(PathBuf::from)
        .ok()
        .or_else(|| current_project_root().map(|root| root.join("infra/repo-access-profile.json")))
        .unwrap_or_else(|| PathBuf::from("infra/repo-access-profile.json"))
}

pub fn current_project_root() -> Option<PathBuf> {
    current_project_root_from(
        env::var("CYNIC_PROJECT_ROOT").map(PathBuf::from).ok(),
        env::current_dir().ok(),
        env::current_exe().ok(),
    )
}

pub fn repo_access_status(
    profile_path: &Path,
    canonical_project_root: Option<&Path>,
) -> RepoAccessStatusReport {
    let entries = load_repo_access_entries(profile_path);
    let canonical = canonical_project_root
        .and_then(resolve_existing_dir)
        .map(path_to_string);

    let mut known_repos = canonical.iter().cloned().collect::<Vec<_>>();
    for entry in &entries {
        if let Some(resolved) = resolve_existing_dir(Path::new(&entry.path)) {
            let resolved_text = path_to_string(resolved);
            if canonical.as_deref() == Some(resolved_text.as_str()) {
                continue;
            }
            known_repos.push(resolved_text);
        }
    }
    known_repos.sort();
    known_repos.dedup();

    let readiness = entries
        .into_iter()
        .map(|entry| {
            let repo_path = PathBuf::from(&entry.path);
            RepoAccessReadiness {
                path: path_to_string(&repo_path),
                owner: entry.owner,
                source: entry.source,
                discovered: entry.discovered,
                added_by: entry.added_by,
                added_at: entry.added_at,
                exists: repo_path.exists(),
            }
        })
        .collect::<Vec<_>>();

    RepoAccessStatusReport {
        schema: REPO_ACCESS_STATUS_SCHEMA,
        mutates: false,
        profile_path: path_to_string(profile_path),
        profile_exists: profile_path.is_file(),
        canonical_project_root: canonical,
        known_repos,
        entries: readiness,
    }
}

pub fn load_repo_access_entries(profile_path: &Path) -> Vec<RepoAccessEntry> {
    let raw = match fs::read_to_string(profile_path) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    let document = match serde_json::from_str::<RepoAccessProfileDocument>(&raw) {
        Ok(document) => document,
        Err(_) => return Vec::new(),
    };
    let repositories = document.repositories.unwrap_or_default();
    let mut entries = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for item in repositories {
        let Some(path) = item
            .path
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let normalized = normalize_existing_or_literal(Path::new(&path));
        if !seen.insert(normalized.clone()) {
            continue;
        }
        entries.push(RepoAccessEntry {
            path: normalized,
            owner: none_if_empty(item.owner),
            added_by: none_if_empty(item.added_by),
            discovered: item.discovered.unwrap_or(false),
            added_at: item
                .added_at
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(default_added_at),
            source: item
                .source
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "sync".to_string()),
        });
    }
    entries
}

fn find_project_root(start: Option<&Path>) -> Option<PathBuf> {
    let start = start?;
    let initial = if start.is_dir() {
        start
    } else {
        start.parent()?
    };
    for candidate in initial.ancestors() {
        if candidate.join("Cargo.toml").is_file()
            && candidate.join("contracts/foundation.json").is_file()
            && candidate.join("infra").is_dir()
            && candidate.join("crates").is_dir()
        {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

fn current_project_root_from(
    project_root_env: Option<PathBuf>,
    current_dir: Option<PathBuf>,
    current_exe: Option<PathBuf>,
) -> Option<PathBuf> {
    project_root_env
        .or_else(|| find_project_root(current_dir.as_deref()))
        .or_else(|| find_project_root(current_exe.as_deref()))
}

fn resolve_existing_dir(path: &Path) -> Option<PathBuf> {
    let resolved = path.canonicalize().ok()?;
    if resolved.is_dir() {
        Some(resolved)
    } else {
        None
    }
}

fn normalize_existing_or_literal(path: &Path) -> String {
    path.canonicalize()
        .ok()
        .map(path_to_string)
        .unwrap_or_else(|| path_to_string(path))
}

fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

fn none_if_empty(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn default_added_at() -> String {
    "1970-01-01T00:00:00+00:00".to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        REPO_ACCESS_STATUS_SCHEMA, current_project_root_from, load_repo_access_entries,
        repo_access_status,
    };
    use std::fs;
    use std::path::Path;

    #[test]
    fn load_repo_access_entries_dedupes_and_normalizes() {
        let tempdir = tempfile::tempdir().unwrap();
        let repo_a = tempdir.path().join("repo-a");
        let repo_b = tempdir.path().join("repo-b");
        fs::create_dir_all(&repo_a).unwrap();
        fs::create_dir_all(&repo_b).unwrap();
        let profile = tempdir.path().join("repo-access.json");
        fs::write(
            &profile,
            format!(
                r#"{{
  "schema": "cynic.runtime_principals.repo_access.v1",
  "repositories": [
    {{"path": "{}", "owner": "alice", "added_by": "apply", "discovered": true, "added_at": "2026-01-01T00:00:00+00:00", "source": "sync"}},
    {{"path": "{}", "owner": "alice", "added_by": "apply", "discovered": true, "added_at": "2026-01-01T00:00:00+00:00", "source": "sync"}},
    {{"path": "{}", "owner": null, "added_by": "", "discovered": false, "added_at": "", "source": ""}}
  ]
}}"#,
                repo_a.display(),
                repo_a.display(),
                repo_b.display()
            ),
        )
        .unwrap();

        let entries = load_repo_access_entries(&profile);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].owner.as_deref(), Some("alice"));
        assert_eq!(entries[1].source, "sync");
        assert_eq!(entries[1].added_at, "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn repo_access_status_reports_known_repos() {
        let tempdir = tempfile::tempdir().unwrap();
        let canonical = tempdir.path().join("canonical");
        let extra = tempdir.path().join("extra");
        fs::create_dir_all(&canonical).unwrap();
        fs::create_dir_all(&extra).unwrap();
        let profile = tempdir.path().join("repo-access.json");
        fs::write(
            &profile,
            format!(
                r#"{{
  "schema": "cynic.runtime_principals.repo_access.v1",
  "repositories": [
    {{"path": "{}", "owner": "alice", "added_by": "apply", "discovered": true, "added_at": "2026-01-01T00:00:00+00:00", "source": "sync"}}
  ]
}}"#,
                extra.display()
            ),
        )
        .unwrap();

        let report = repo_access_status(&profile, Some(&canonical));
        assert_eq!(report.schema, REPO_ACCESS_STATUS_SCHEMA);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.known_repos.len(), 2);
        assert!(
            report
                .known_repos
                .iter()
                .any(|item| item == &canonical.to_string_lossy())
        );
        assert!(
            report
                .known_repos
                .iter()
                .any(|item| item == &extra.to_string_lossy())
        );
    }

    #[test]
    fn current_project_root_prefers_env() {
        let tempdir = tempfile::tempdir().unwrap();
        let root = tempdir.path().join("root");
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::create_dir_all(root.join("crates")).unwrap();
        fs::create_dir_all(root.join("infra")).unwrap();
        fs::write(root.join("Cargo.toml"), "").unwrap();
        fs::write(root.join("contracts/foundation.json"), "{}").unwrap();
        let current = current_project_root_from(Some(root.clone()), None, None);
        assert_eq!(current.as_deref(), Some(Path::new(&root)));
    }
}
