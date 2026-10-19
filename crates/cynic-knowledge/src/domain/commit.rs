use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use serde::{Deserialize, Serialize};

pub const GIT_COMMIT_SCHEMA_V1: &str = "cynic.knowledge.git_commit.v1";

/// Enregistrement structuré d'un commit Git.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommitRecord {
    pub schema: String,
    pub hash: String,
    pub parent_hashes: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub subject: String,
    pub body: String,
    pub files_changed: Vec<String>,
}

impl GitCommitRecord {
    pub fn new(
        hash: String,
        parent_hashes: Vec<String>,
        author_name: String,
        author_email: String,
        timestamp: i64,
        message: String,
        files_changed: Vec<String>,
    ) -> Self {
        let trimmed_msg = message.trim();
        let mut lines = trimmed_msg.lines();
        let subject = lines.next().unwrap_or("").trim().to_string();
        let body = lines.collect::<Vec<&str>>().join("\n").trim().to_string();

        Self {
            schema: GIT_COMMIT_SCHEMA_V1.to_string(),
            hash,
            parent_hashes,
            author_name,
            author_email,
            timestamp,
            subject,
            body,
            files_changed,
        }
    }

    /// Convertit le commit en DocumentChunk pour l'index lexical BM25.
    pub fn to_chunk(&self) -> DocumentChunk {
        let short_hash = if self.hash.len() >= 8 {
            &self.hash[..8]
        } else {
            &self.hash
        };

        let hierarchy = vec![
            "VCS History".to_string(),
            format!("Commit {} ({})", short_hash, self.author_name),
            self.subject.clone(),
        ];

        let mut content = format!(
            "Subject: {}\nAuthor: {} <{}>\nTimestamp: {}\n",
            self.subject, self.author_name, self.author_email, self.timestamp
        );
        if !self.body.is_empty() {
            content.push_str(&format!("\nBody:\n{}\n", self.body));
        }
        if !self.files_changed.is_empty() {
            content.push_str("\nFiles Changed:\n");
            for file in &self.files_changed {
                content.push_str(&format!("- {}\n", file));
            }
        }

        DocumentChunk::new(
            format!("git://commit/{}", self.hash),
            1,
            content.lines().count(),
            hierarchy,
            content,
            AuthorityTier::L2Design,
        )
    }
}
