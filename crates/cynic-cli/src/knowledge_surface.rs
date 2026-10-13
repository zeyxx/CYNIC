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
        /// Also index recent Git commits for VCS history search.
        #[arg(long, default_value = "0")]
        commits: usize,
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
    /// Inspect recent VCS Git history records.
    Commits {
        /// Number of recent commits to inspect.
        #[arg(long, default_value = "10")]
        limit: usize,
        /// Output commits as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Inspect the architectural geometry of crates across layers.
    Crates {
        /// Filter by architectural layer (e.g. L0, L1, L2, L3, L4).
        #[arg(long)]
        layer: Option<String>,
        /// Output crate geometry as JSON.
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
        KnowCommand::Search {
            query,
            limit,
            commits,
            json,
        } => {
            let index = if commits > 0 {
                cynic_knowledge::index_workspace_with_commits(&repo_root, commits)
            } else {
                index
            };
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
        KnowCommand::Commits { limit, json } => {
            match cynic_knowledge::extract_git_history(&repo_root, limit) {
                Ok(commits) => {
                    if json {
                        crate::emit_json(&commits, false)
                    } else {
                        println!("=== CYNIC VCS History (Last {} Commits) ===", commits.len());
                        for c in &commits {
                            let short_hash = if c.hash.len() >= 8 {
                                &c.hash[..8]
                            } else {
                                &c.hash
                            };
                            println!(
                                "\n[{}] {} — {} ({})",
                                short_hash, c.subject, c.author_name, c.timestamp
                            );
                            if !c.files_changed.is_empty() {
                                println!("    Touched ({} files):", c.files_changed.len());
                                for f in c.files_changed.iter().take(5) {
                                    println!("      - {}", f);
                                }
                                if c.files_changed.len() > 5 {
                                    println!("      - ... (+{} files)", c.files_changed.len() - 5);
                                }
                            }
                        }
                        ExitCode::SUCCESS
                    }
                }
                Err(err) => {
                    eprintln!("Failed to extract git history: {}", err);
                    ExitCode::FAILURE
                }
            }
        }
        KnowCommand::Crates { layer, json } => {
            match cynic_knowledge::load_workspace_geometry(&repo_root) {
                Ok(geom) => {
                    if json {
                        crate::emit_json(&geom, false)
                    } else {
                        println!("=== CYNIC Workspace Crate Geometry ({}) ===", geom.schema);
                        println!("  Total Crates : {}", geom.total_crates);
                        println!("  Total LOC    : {}", geom.total_loc);

                        let layers = ["L0", "L1", "L2", "L3", "L4"];
                        for l in layers {
                            if let Some(ref filter_l) = layer {
                                if filter_l != l {
                                    continue;
                                }
                            }
                            let in_layer = geom.crates_in_layer(l);
                            if !in_layer.is_empty() {
                                println!("\n  [Layer {} — {} crates]:", l, in_layer.len());
                                for c in in_layer {
                                    println!(
                                        "    - {:<30} {:>6} LOC (fan-in: {:>2}, fan-out: {:>2})",
                                        c.name,
                                        c.loc,
                                        c.fan_in(),
                                        c.fan_out()
                                    );
                                }
                            }
                        }
                        ExitCode::SUCCESS
                    }
                }
                Err(err) => {
                    eprintln!("Failed to load workspace geometry: {}", err);
                    ExitCode::FAILURE
                }
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
