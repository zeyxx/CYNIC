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
    /// Ask a question to the knowledge oracle with strict epistemic grounding.
    Ask {
        /// The question to answer.
        question: String,
        /// Maximum number of sources to retrieve.
        #[arg(long, default_value = "4")]
        limit: usize,
        /// Also index recent Git commits for provenance.
        #[arg(long, default_value = "20")]
        commits: usize,
        /// Only output the bounded epistemic prompt without invoking local LLM.
        #[arg(long)]
        prompt_only: bool,
        /// Optional custom OpenAI-compatible endpoint (defaults to http://127.0.0.1:8080/v1/chat/completions).
        #[arg(long)]
        endpoint: Option<String>,
        /// Model name for completion.
        #[arg(long, default_value = "default")]
        model: String,
        /// Output response as JSON.
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
        KnowCommand::Ask {
            question,
            limit,
            commits,
            prompt_only,
            endpoint,
            model,
            json,
        } => {
            let index = cynic_knowledge::index_workspace_with_commits(&repo_root, commits);
            let hits = index.search(&question, limit);
            let synthesis_prompt = cynic_knowledge::SynthesisPromptBuilder::build(&question, &hits);

            if prompt_only {
                if json {
                    crate::emit_json(&synthesis_prompt, false)
                } else {
                    println!(
                        "=== CYNIC Epistemic Prompt ({}) ===",
                        synthesis_prompt.schema
                    );
                    println!(
                        "\n[SYSTEM INSTRUCTION]\n{}",
                        synthesis_prompt.system_instruction
                    );
                    println!("\n[USER PROMPT]\n{}", synthesis_prompt.user_prompt);
                    ExitCode::SUCCESS
                }
            } else {
                let target_endpoint = endpoint.unwrap_or_else(
                    cynic_knowledge::SynthesisPromptBuilder::resolve_local_inference_endpoint,
                );

                // Attempt local LLM call via reqwest
                let payload = serde_json::json!({
                    "model": model,
                    "messages": [
                        {"role": "system", "content": synthesis_prompt.system_instruction},
                        {"role": "user", "content": synthesis_prompt.user_prompt}
                    ],
                    "temperature": 0.0,
                    "max_tokens": 300
                });

                let mut answer_text = None;
                let mut duration_ms = 0u128;
                let mut error_msg = None;
                tokio::task::block_in_place(|| {
                    let client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(90))
                        .build();

                    // Resolve API key from env or canonical CynicPaths
                    let api_key = std::env::var("CYNIC_LLAMA_API_KEY")
                        .or_else(|_| std::env::var("CYNIC_API_KEY"))
                        .ok()
                        .or_else(|| {
                            cynic_types::CynicPaths::user_config_dir()
                                .and_then(|p| std::fs::read_to_string(p.join("llama-api-key")).ok())
                        })
                        .or_else(|| {
                            std::fs::read_to_string(cynic_types::CynicPaths::secret_path(
                                "llama-api-key",
                            ))
                            .ok()
                        })
                        .or_else(|| std::fs::read_to_string("/etc/cynic/llama-api-key").ok())
                        .map(|k| k.trim().to_string());

                    if let Ok(client) = client {
                        let mut req = client.post(&target_endpoint).json(&payload);
                        if let Some(ref key) = api_key {
                            req = req.header("Authorization", format!("Bearer {}", key));
                        }
                        let start = std::time::Instant::now();
                        match req.send() {
                            Ok(resp) => {
                                duration_ms = start.elapsed().as_millis();
                                if resp.status().is_success() {
                                    if let Ok(body) = resp.json::<serde_json::Value>() {
                                        if let Some(content) =
                                            body["choices"][0]["message"]["content"].as_str()
                                        {
                                            answer_text = Some(content.to_string());
                                        }
                                    }
                                } else {
                                    error_msg = Some(format!("HTTP {}", resp.status().as_u16()));
                                }
                            }
                            Err(e) => {
                                error_msg = Some(format!("Connection error: {}", e));
                            }
                        }
                    }
                });

                if json {
                    let out = serde_json::json!({
                        "question": question,
                        "answer": answer_text,
                        "endpoint": target_endpoint,
                        "sources": synthesis_prompt.sources,
                        "prompt": synthesis_prompt
                    });
                    crate::emit_json(&out, false)
                } else {
                    println!("=== CYNIC Knowledge Synthesis Oracle ===");
                    println!("Question : {}", question);
                    println!(
                        "Sources  : {} chunks retenus",
                        synthesis_prompt.sources.len()
                    );
                    for (i, s) in synthesis_prompt.sources.iter().enumerate() {
                        println!(
                            "  [{}] {}:{}-{} ({:?}, score: {:.2})",
                            i + 1,
                            s.source_path,
                            s.line_start,
                            s.line_end,
                            s.authority,
                            s.score
                        );
                    }

                    match answer_text {
                        Some(ans) => {
                            println!(
                                "\n=== Réponse Synthétisée ({} ms) ===\n{}",
                                duration_ms, ans
                            );
                        }
                        None => {
                            let err_suffix =
                                error_msg.map(|e| format!(" ({})", e)).unwrap_or_default();
                            println!(
                                "\n[!] Serveur d'inférence ({}) injoignable ou en erreur{}.",
                                target_endpoint, err_suffix
                            );
                            println!(
                                "    Pour exécuter avec un agent, utilisez l'option `--prompt-only`."
                            );
                        }
                    }
                    ExitCode::SUCCESS
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
