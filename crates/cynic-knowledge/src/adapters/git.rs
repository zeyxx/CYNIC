use crate::domain::commit::GitCommitRecord;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitExtractError {
    #[error("Failed to execute git command: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git command exited with non-zero status: {0}")]
    Failed(String),
}

/// Extrait l'historique récent des commits Git du workspace.
pub fn extract_git_history<P: AsRef<Path>>(
    repo_root: P,
    max_commits: usize,
) -> Result<Vec<GitCommitRecord>, GitExtractError> {
    let output = Command::new("git")
        .current_dir(repo_root.as_ref())
        .args([
            "log",
            "-n",
            &max_commits.to_string(),
            "--name-only",
            "--pretty=format:COMMIT_START%n%H%n%P%n%an%n%ae%n%at%n%B%nCOMMIT_FILES",
        ])
        .output()?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(GitExtractError::Failed(err_msg));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_git_log_output(&text))
}

/// Parseur pur sans I/O du flux délimité git log.
pub fn parse_git_log_output(text: &str) -> Vec<GitCommitRecord> {
    let mut records = Vec::new();
    let sections = text.split("COMMIT_START\n");

    for section in sections {
        let trimmed = section.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Le format attendu après COMMIT_START est :
        // hash
        // parent_hashes
        // author_name
        // author_email
        // timestamp
        // body...
        // COMMIT_FILES
        // files...
        let parts: Vec<&str> = trimmed.split("COMMIT_FILES").collect();
        let meta_and_msg = parts[0].trim();
        let files_part = parts.get(1).map(|s| s.trim()).unwrap_or("");

        let mut lines = meta_and_msg.lines();
        let hash = match lines.next() {
            Some(h) if !h.trim().is_empty() => h.trim().to_string(),
            _ => continue,
        };

        let parents_raw = lines.next().unwrap_or("").trim();
        let parent_hashes: Vec<String> = if parents_raw.is_empty() {
            Vec::new()
        } else {
            parents_raw
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        };

        let author_name = lines.next().unwrap_or("").trim().to_string();
        let author_email = lines.next().unwrap_or("").trim().to_string();
        let timestamp: i64 = lines.next().unwrap_or("0").trim().parse().unwrap_or(0);

        let message = lines.collect::<Vec<&str>>().join("\n").trim().to_string();

        let files_changed: Vec<String> = files_part
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        records.push(GitCommitRecord::new(
            hash,
            parent_hashes,
            author_name,
            author_email,
            timestamp,
            message,
            files_changed,
        ));
    }

    records
}
