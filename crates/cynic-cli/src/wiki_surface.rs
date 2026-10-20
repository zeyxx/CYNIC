//! Read-only projection of the CYNICpedia article registry (`infra/cynicpedia.json`).
//!
//! WHY: CYNICpedia is the canonical index of knowledge articles (docs, contracts,
//! traces, designs) with provenance. This surface projects it for operators and
//! reuses `cynic-knowledge` for lexical search, mapping hits back to registered
//! articles.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{emit_json, fail, rust_source};

#[derive(Subcommand)]
pub enum WikiCommand {
    /// List registered CYNICpedia articles.
    List {
        /// Filter by article kind (doc/contract/trace/design).
        #[arg(long)]
        kind: Option<String>,
        /// Filter by article status (draft/published/deprecated).
        #[arg(long)]
        status: Option<String>,
        /// Path to the CYNICpedia registry (default: infra/cynicpedia.json).
        #[arg(long)]
        registry: Option<String>,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Search articles using cynic-knowledge lexical retrieval.
    Search {
        /// The query string.
        query: String,
        /// Maximum number of results.
        #[arg(long, default_value = "10")]
        limit: usize,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show a single registered article by id.
    Show {
        /// Article id.
        article_id: String,
        /// Path to the CYNICpedia registry (default: infra/cynicpedia.json).
        #[arg(long)]
        registry: Option<String>,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CynicpediaRegistry {
    schema: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    purpose: String,
    articles: Vec<Article>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Article {
    article_id: String,
    title: String,
    path: String,
    kind: String,
    provenance: String,
    status: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct WikiListReport {
    schema: &'static str,
    registry_path: String,
    article_count: usize,
    articles: Vec<Article>,
}

#[derive(Debug, Serialize)]
struct WikiSearchHitView {
    article_id: String,
    title: String,
    path: String,
    kind: String,
    status: String,
    score: f64,
}

#[derive(Debug, Serialize)]
struct WikiShowReport {
    schema: &'static str,
    registry_path: String,
    article: Article,
    file_exists: bool,
}

pub fn execute_wiki_command(command: WikiCommand) -> ExitCode {
    match command {
        WikiCommand::List {
            kind,
            status,
            registry,
            json,
        } => {
            let root = rust_source::workspace_root();
            let default_path = root.join("infra/cynicpedia.json");
            let registry_path = registry
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_path);
            let report = list_articles(&registry_path, kind.as_deref(), status.as_deref());
            if json {
                emit_json(&report, false)
            } else {
                render_list(&report);
                ExitCode::SUCCESS
            }
        }
        WikiCommand::Search { query, limit, json } => {
            let root = rust_source::workspace_root();
            let registry_path = root.join("infra/cynicpedia.json");
            let registry = load_registry(&registry_path);
            let articles = registry.map(|r| r.articles).unwrap_or_default();
            let index = cynic_knowledge::index_workspace(&root);
            let hits = index.search(&query, limit * 4);
            // Map knowledge hits to registered articles (dedup by article_id).
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for hit in hits {
                if let Some(article) = articles.iter().find(|a| a.path == hit.chunk.source_path) {
                    if seen.insert(article.article_id.clone()) {
                        out.push(WikiSearchHitView {
                            article_id: article.article_id.clone(),
                            title: article.title.clone(),
                            path: article.path.clone(),
                            kind: article.kind.clone(),
                            status: article.status.clone(),
                            score: hit.score,
                        });
                    }
                }
                if out.len() >= limit {
                    break;
                }
            }
            if json {
                emit_json(&out, false)
            } else {
                if out.is_empty() {
                    println!("No wiki articles found matching query: \"{}\"", query);
                    return ExitCode::SUCCESS;
                }
                println!("=== Found {} wiki articles matching \"{}\" ===", out.len(), query);
                for (idx, h) in out.iter().enumerate() {
                    println!(
                        "\n[{}] {} ({}, score: {:.2})",
                        idx + 1,
                        h.title,
                        h.kind,
                        h.score
                    );
                    println!("    id:   {}", h.article_id);
                    println!("    path: {}", h.path);
                }
                ExitCode::SUCCESS
            }
        }
        WikiCommand::Show {
            article_id,
            registry,
            json,
        } => {
            let root = rust_source::workspace_root();
            let default_path = root.join("infra/cynicpedia.json");
            let registry_path = registry
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_path);
            match show_article(&registry_path, &article_id) {
                Ok(report) => {
                    if json {
                        emit_json(&report, false)
                    } else {
                        render_show(&report);
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => fail(e),
            }
        }
    }
}

fn load_registry(path: &Path) -> Option<CynicpediaRegistry> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn list_articles(path: &Path, kind: Option<&str>, status: Option<&str>) -> WikiListReport {
    let registry_path_str = path.display().to_string();
    let registry = load_registry(path);
    let mut articles = registry.map(|r| r.articles).unwrap_or_default();
    if let Some(k) = kind {
        articles.retain(|a| a.kind == k);
    }
    if let Some(s) = status {
        articles.retain(|a| a.status == s);
    }
    WikiListReport {
        schema: "cynic.cli.wiki_list.v1",
        registry_path: registry_path_str,
        article_count: articles.len(),
        articles,
    }
}

fn show_article(path: &Path, article_id: &str) -> Result<WikiShowReport, String> {
    let registry = load_registry(path).ok_or_else(|| format!("cannot load registry: {}", path.display()))?;
    let article = registry
        .articles
        .into_iter()
        .find(|a| a.article_id == article_id)
        .ok_or_else(|| format!("article not found: {article_id}"))?;
    let root = rust_source::workspace_root();
    let file_exists = root.join(&article.path).exists();
    Ok(WikiShowReport {
        schema: "cynic.cli.wiki_show.v1",
        registry_path: path.display().to_string(),
        article,
        file_exists,
    })
}

fn render_list(report: &WikiListReport) {
    println!("=== CYNICpedia ({} articles) ===", report.article_count);
    println!("  Registry: {}", report.registry_path);
    for a in &report.articles {
        println!("  - [{}] {} ({}) — {}", a.article_id, a.title, a.kind, a.path);
    }
}

fn render_show(report: &WikiShowReport) {
    let a = &report.article;
    println!("=== CYNICpedia Article ===");
    println!("  id:         {}", a.article_id);
    println!("  title:      {}", a.title);
    println!("  path:       {}", a.path);
    println!("  kind:       {}", a.kind);
    println!("  status:     {}", a.status);
    println!("  provenance: {}", a.provenance);
    println!("  created:    {}", a.created_at);
    println!("  updated:    {}", a.updated_at);
    println!("  file exists: {}", report.file_exists);
}
