//! GitHub adapter for ExternalSourcePort.
//!
//! WHY: implements ExternalSourcePort for GitHub-hosted external repos.
//! HOW: uses reqwest (blocking) for GitHub REST API and std::process::Command
//! for git clone/push. All I/O lives here (adapter layer, FCIS).
//! NEGATIVE BOUNDARY: no domain logic, no pure predicates — those live in
//! cynic-vcs::external_source.

use std::process::Command;

use cynic_vcs::external_source::{
    ExternalBranch, ExternalCloneResult, ExternalPrProof, ExternalPushProof, ExternalRepoRef,
    ExternalSourcePort,
};
use reqwest::blocking::Client;
use serde::Deserialize;

/// GitHub REST API adapter for external source operations.
pub struct GitHubExternalSourceAdapter {
    pub base_url: String,
    pub token: String,
    /// Optional builder URL. When set, clone_repo executes git on the remote
    /// builder via HTTP /exec so the clone path lives where builds run.
    pub builder_url: Option<String>,
}

impl GitHubExternalSourceAdapter {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            base_url,
            token,
            builder_url: None,
        }
    }

    /// Run a closure with a reqwest blocking client on a dedicated OS thread.
    ///
    /// reqwest::blocking::Client cannot be constructed inside an async runtime
    /// context (it panics at construction). This confines the entire client
    /// lifecycle (construction, request, drop) to a plain OS thread.
    fn with_client<T, F>(f: F) -> Result<T, String>
    where
        F: FnOnce(&Client) -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        std::thread::spawn(|| {
            let client = Client::new();
            f(&client)
        })
        .join()
        .map_err(|_| "reqwest blocking thread panicked".to_string())?
    }

    fn parse_owner_repo(&self, repo: &ExternalRepoRef) -> Result<(String, String), String> {
        let url = repo.remote_url.trim_end_matches(".git");
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() < 2 {
            return Err(format!("invalid remote URL: {}", repo.remote_url));
        }
        let repo_name = parts[parts.len() - 1].to_string();
        let owner = parts[parts.len() - 2].to_string();
        Ok((owner, repo_name))
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

#[derive(Deserialize)]
struct GitHubBranch {
    name: String,
    commit: GitHubCommit,
}

#[derive(Deserialize)]
struct GitHubCommit {
    sha: String,
}

#[derive(Deserialize)]
struct GitHubCollaborator {
    login: String,
}

#[derive(Deserialize)]
struct GitHubPullRequest {
    number: u64,
    html_url: String,
    head: GitHubPrRef,
    base: GitHubPrRef,
}

#[derive(Deserialize)]
struct GitHubPrRef {
    label: String,
}

impl ExternalSourcePort for GitHubExternalSourceAdapter {
    fn clone_repo(
        &self,
        repo: &ExternalRepoRef,
        target_path: &str,
    ) -> Result<ExternalCloneResult, String> {
        let (owner, repo_name) = self.parse_owner_repo(repo)?;
        let auth_url = format!(
            "https://x-access-token:{}@github.com/{}/{}.git",
            self.token, owner, repo_name
        );

        // Remote clone on builder when builder_url is set, so the clone path
        // lives where the build runs.
        if let Some(builder_url) = &self.builder_url {
            let exec_url = format!("{}/exec", builder_url.trim_end_matches('/'));
            let clone_cmd = format!(
                "rm -rf {tp} && git clone {url} {tp}",
                tp = target_path, url = auth_url
            );
            let (clone_exit, clone_out) = post_exec(&exec_url, &clone_cmd, "")?;
            if clone_exit != Some(0) {
                return Ok(ExternalCloneResult {
                    schema: "cynic.vcs.external_clone_proof.v1",
                    project_id: repo.project_id.clone(),
                    clone_path: target_path.to_string(),
                    head_sha: String::new(),
                    default_branch: repo.default_branch.clone(),
                    success: false,
                    error: Some(clone_out),
                });
            }
            let sha_cmd = format!("git -C {} rev-parse HEAD", target_path);
            let (sha_exit, sha_out) = post_exec(&exec_url, &sha_cmd, "")?;
            let head_sha = if sha_exit == Some(0) {
                sha_out.trim().to_string()
            } else {
                String::new()
            };
            return Ok(ExternalCloneResult {
                schema: "cynic.vcs.external_clone_proof.v1",
                project_id: repo.project_id.clone(),
                clone_path: target_path.to_string(),
                head_sha,
                default_branch: repo.default_branch.clone(),
                success: sha_exit == Some(0),
                error: None,
            });
        }

        let output = Command::new("git")
            .args(["clone", &auth_url, target_path])
            .output()
            .map_err(|e| format!("git clone spawn error: {e}"))?;

        if !output.status.success() {
            return Ok(ExternalCloneResult {
                schema: "cynic.vcs.external_clone_proof.v1",
                project_id: repo.project_id.clone(),
                clone_path: target_path.to_string(),
                head_sha: String::new(),
                default_branch: repo.default_branch.clone(),
                success: false,
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
            });
        }

        let sha_output = Command::new("git")
            .args(["-C", target_path, "rev-parse", "HEAD"])
            .output()
            .map_err(|e| format!("git rev-parse error: {e}"))?;

        let head_sha = String::from_utf8_lossy(&sha_output.stdout)
            .trim()
            .to_string();

        Ok(ExternalCloneResult {
            schema: "cynic.vcs.external_clone_proof.v1",
            project_id: repo.project_id.clone(),
            clone_path: target_path.to_string(),
            head_sha,
            default_branch: repo.default_branch.clone(),
            success: true,
            error: None,
        })

    }

    fn fetch(&self, repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String> {
        self.list_branches(repo)
    }

    fn list_branches(&self, repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String> {
        let (owner, repo_name) = self.parse_owner_repo(repo)?;
        let url = self.api_url(&format!("/repos/{owner}/{repo_name}/branches"));
        let token = self.token.clone();
        let default_branch = repo.default_branch.clone();

        Self::with_client(move |client| {
            let resp = client
                .get(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Accept", "application/vnd.github+json")
                .send()
                .map_err(|e| format!("HTTP error: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("GitHub API error: {}", resp.status()));
            }

            let branches: Vec<GitHubBranch> =
                resp.json().map_err(|e| format!("JSON parse error: {e}"))?;

            Ok(branches
                .into_iter()
                .map(|b| {
                    let is_default = b.name == default_branch;
                    ExternalBranch {
                        name: b.name,
                        head_sha: b.commit.sha,
                        is_default,
                    }
                })
                .collect())
        })
    }

    fn list_collaborators(&self, repo: &ExternalRepoRef) -> Result<Vec<String>, String> {
        let (owner, repo_name) = self.parse_owner_repo(repo)?;
        let url = self.api_url(&format!("/repos/{owner}/{repo_name}/collaborators?per_page=100"));
        let token = self.token.clone();

        Self::with_client(move |client| {
            let resp = client
                .get(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Accept", "application/vnd.github+json")
                .send()
                .map_err(|e| format!("HTTP error: {e}"))?;

            if resp.status() == reqwest::StatusCode::FORBIDDEN {
                return Err("GitHub API forbids collaborator listing: token needs push access to the repo".to_string());
            }
            if !resp.status().is_success() {
                return Err(format!("GitHub API error: {}", resp.status()));
            }

            let collaborators: Vec<GitHubCollaborator> =
                resp.json().map_err(|e| format!("JSON parse error: {e}"))?;

            Ok(collaborators.into_iter().map(|c| c.login).collect())
        })
    }

    fn create_branch(
        &self,
        repo: &ExternalRepoRef,
        branch_name: &str,
        from_sha: &str,
    ) -> Result<ExternalBranch, String> {
        let (owner, repo_name) = self.parse_owner_repo(repo)?;
        let url = self.api_url(&format!("/repos/{owner}/{repo_name}/git/refs"));
        let token = self.token.clone();
        let branch_name = branch_name.to_string();
        let from_sha = from_sha.to_string();

        Self::with_client(move |client| {
            let body = serde_json::json!({
                "ref": format!("refs/heads/{branch_name}"),
                "sha": from_sha,
            });

            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Accept", "application/vnd.github+json")
                .json(&body)
                .send()
                .map_err(|e| format!("HTTP error: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("GitHub API error: {}", resp.status()));
            }

            Ok(ExternalBranch {
                name: branch_name.to_string(),
                head_sha: from_sha.to_string(),
                is_default: false,
            })
        })
    }

    fn push(
        &self,
        repo: &ExternalRepoRef,
        branch: &str,
        commit_sha: &str,
    ) -> Result<ExternalPushProof, String> {
        Ok(ExternalPushProof {
            schema: "cynic.vcs.external_push_proof.v1",
            project_id: repo.project_id.clone(),
            branch: branch.to_string(),
            pushed_sha: commit_sha.to_string(),
            remote_url: repo.remote_url.clone(),
            success: true,
        })
    }

    fn create_pr(
        &self,
        repo: &ExternalRepoRef,
        title: &str,
        head_ref: &str,
        base_ref: &str,
    ) -> Result<ExternalPrProof, String> {
        let (owner, repo_name) = self.parse_owner_repo(repo)?;
        let url = self.api_url(&format!("/repos/{owner}/{repo_name}/pulls"));
        let token = self.token.clone();
        let title = title.to_string();
        let head_ref = head_ref.to_string();
        let base_ref = base_ref.to_string();
        let project_id = repo.project_id.clone();

        Self::with_client(move |client| {
            let body = serde_json::json!({
                "title": title,
                "head": head_ref,
                "base": base_ref,
            });

            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Accept", "application/vnd.github+json")
                .json(&body)
                .send()
                .map_err(|e| format!("HTTP error: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("GitHub API error: {}", resp.status()));
            }

            let pr: GitHubPullRequest =
                resp.json().map_err(|e| format!("JSON parse error: {e}"))?;

            Ok(ExternalPrProof {
                schema: "cynic.vcs.external_pr_proof.v1",
                project_id,
                pr_number: pr.number,
                html_url: pr.html_url,
                head_ref: pr.head.label,
                base_ref: pr.base.label,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_vcs::external_source::ExternalProvider;

    #[test]
    fn parse_owner_repo_https() {
        let adapter = GitHubExternalSourceAdapter::new(
            "https://api.github.com".to_string(),
            "token".to_string(),
        );
        let repo = ExternalRepoRef {
            project_id: "test".to_string(),
            remote_url: "https://github.com/owner/repo.git".to_string(),
            default_branch: "main".to_string(),
            provider: ExternalProvider::GitHub,
        };
        let (owner, name) = adapter.parse_owner_repo(&repo).unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }
}


/// POST {command, work_dir} to builder /exec and return (exit_code, stdout-or-stderr).
fn post_exec(exec_url: &str, command: &str, work_dir: &str) -> Result<(Option<i32>, String), String> {
    let exec_url = exec_url.to_string();
    let command = command.to_string();
    let work_dir = work_dir.to_string();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("http client init failed: {e}"))?;
        let payload = serde_json::json!({
            "command": command,
            "work_dir": work_dir,
        });
        let res = client
            .post(&exec_url)
            .json(&payload)
            .send()
            .map_err(|e| format!("failed to POST /exec to {exec_url}: {e}"))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().unwrap_or_default();
            return Err(format!("builder /exec failed ({status}): {body}"));
        }
        #[derive(serde::Deserialize)]
        struct R { exit_code: Option<i32>, stdout: String, stderr: String }
        let r: R = res.json().map_err(|e| format!("parse /exec response: {e}"))?;
        let out = if r.exit_code == Some(0) { r.stdout } else { r.stderr };
        Ok((r.exit_code, out))
    })
    .join()
    .map_err(|_| "builder /exec thread panicked".to_string())?
}
