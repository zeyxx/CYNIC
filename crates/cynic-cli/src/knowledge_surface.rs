use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Subcommand)]
pub enum KnowCommand {
    /// Deterministic lexical search across documentation and normative contracts.
    Search {
        /// The query string (exact symbol, phrase, or invariant).
        query: String,
        /// Maximum number of results to display.
        #[arg(long, default_value = "3")]
        limit: usize,
        /// Output search results as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Index and summarize all documentation and contracts in the workspace.
    Index {
        /// Output indexing summary as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Export complete hierarchical corpus manifest (100% of Markdown files AST).
    Manifest {
        /// Output full CorpusManifest as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Inspect the knowledge graph (cross-document links, contract enforcements, broken refs).
    Graph {
        /// Output full KnowledgeGraph as JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
pub struct KnowledgeSearchHitView {
    pub chunk_id: String,
    pub source_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub section: String,
    pub authority: String,
    pub score: f64,
    pub content_preview: String,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeIndexSummary {
    pub chunk_count: usize,
    pub status: String,
}

pub async fn execute_know_command(command: KnowCommand) -> ExitCode {
    // Detect repository root from current working directory or environment
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let repo_root = find_repo_root(&cwd).unwrap_or(cwd);

    let index = cynic_knowledge::index_workspace(&repo_root);

    match command {
        KnowCommand::Index { json } => {
            let summary = KnowledgeIndexSummary {
                chunk_count: index.chunk_count(),
                status: "indexed".to_string(),
            };

            if json {
                crate::emit_json(&summary, false)
            } else {
                println!(
                    "Knowledge Index: {} chunks indexed from docs/ and contracts/ at {}",
                    summary.chunk_count,
                    repo_root.display()
                );
                ExitCode::SUCCESS
            }
        }
        KnowCommand::Manifest { json } => {
            let manifest = cynic_knowledge::scan_corpus_manifest(&repo_root);
            if json {
                crate::emit_json(&manifest, false)
            } else {
                println!("=== CYNIC Corpus Manifest ({}) ===", manifest.schema);
                println!("  Corpus Hash       : {}", manifest.corpus_hash);
                println!("  Total Documents   : {}", manifest.total_documents);
                println!("  Total Sections    : {}", manifest.total_sections);
                println!("  Total Code Blocks : {}", manifest.total_code_blocks);
                println!("  Total Bytes       : {}", manifest.total_bytes);
                ExitCode::SUCCESS
            }
        }
        KnowCommand::Graph { json } => {
            let manifest = cynic_knowledge::scan_corpus_manifest(&repo_root);
            let graph = cynic_knowledge::KnowledgeGraph::build_from_manifest(&repo_root, &manifest);
            if json {
                crate::emit_json(&graph, false)
            } else {
                println!("=== CYNIC Knowledge Graph ===");
                println!("  Total Nodes       : {}", graph.nodes.len());
                println!("  Total Edges       : {}", graph.edges.len());
                let doc_citations = graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == cynic_knowledge::KnowledgeEdgeKind::CitesDocument)
                    .count();
                let contract_edges = graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == cynic_knowledge::KnowledgeEdgeKind::EnforcesContract)
                    .count();
                println!("  Document Citations: {}", doc_citations);
                println!("  Contract Enforces : {}", contract_edges);
                println!("  Broken References : {}", graph.broken_references.len());
                if !graph.broken_references.is_empty() {
                    println!("\n  [!] Broken References Found:");
                    for b in &graph.broken_references {
                        println!("    {} -> {}", b.source_doc, b.target_raw);
                    }
                }
                ExitCode::SUCCESS
            }
        }
        KnowCommand::Search { query, limit, json } => {
            let hits = index.search(&query, limit);

            if json {
                let views: Vec<KnowledgeSearchHitView> = hits
                    .iter()
                    .map(|h| KnowledgeSearchHitView {
                        chunk_id: h.chunk.chunk_id.clone(),
                        source_path: h.chunk.source_path.clone(),
                        line_start: h.chunk.line_start,
                        line_end: h.chunk.line_end,
                        section: h.chunk.title(),
                        authority: format!("{:?}", h.chunk.authority),
                        score: h.score,
                        content_preview: h
                            .chunk
                            .content
                            .lines()
                            .take(5)
                            .collect::<Vec<_>>()
                            .join("\n"),
                    })
                    .collect();
                crate::emit_json(&views, false)
            } else {
                if hits.is_empty() {
                    println!("No knowledge chunks found matching query: \"{}\"", query);
                    return ExitCode::SUCCESS;
                }

                println!(
                    "=== Found {} knowledge chunks matching \"{}\" ===",
                    hits.len(),
                    query
                );
                for (idx, hit) in hits.iter().enumerate() {
                    println!(
                        "\n[{}] {} (score: {:.2}, {:?})",
                        idx + 1,
                        hit.chunk.title(),
                        hit.score,
                        hit.chunk.authority
                    );
                    println!(
                        "    File: {}:{}-{}",
                        hit.chunk.source_path, hit.chunk.line_start, hit.chunk.line_end
                    );
                    println!("    ---");
                    for line in hit.chunk.content.lines().take(6) {
                        println!("    {}", line);
                    }
                    if hit.chunk.content.lines().count() > 6 {
                        println!("    [...]");
                    }
                }
                ExitCode::SUCCESS
            }
        }
    }
}

fn find_repo_root(start: &std::path::Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() || current.join("Cargo.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}
