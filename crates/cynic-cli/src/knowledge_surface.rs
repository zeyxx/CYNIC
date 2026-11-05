use clap::Subcommand;
use cynic_knowledge::service::*;
use serde::Serialize;
use serde_json;
use cynic_types;
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
    /// Review HITL grade proposals against the deterministic gate, and
    /// optionally merge accepted ones into the retrieval fixture.
    Grades {
        /// JSON file with an array of grade proposals.
        file: PathBuf,
        /// Merge accepted proposals into the fixture (otherwise dry-run).
        #[arg(long)]
        apply: bool,
        /// Output verdicts as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Harvest live verifiable facts and generate the public epistemic surface (HTML + Markdown).
    Surface {
        /// Optional destination directory for public_surface.html and public_surface.md [default: docs/public]
        #[arg(long)]
        out_dir: Option<PathBuf>,
        /// Check every event proof_reference against the local git history and re-run every claim
        /// verify_command against its published number. Exits non-zero on an unresolved reference
        /// or a mismatch; nothing is written.
        #[arg(long)]
        verify: bool,
        /// Check that the already published facts.live.json in --out-dir is at most this many hours
        /// old. Exits non-zero if it is stale, in the future or unreadable; nothing is regenerated.
        #[arg(long)]
        max_age_hours: Option<u64>,
        /// Output live fact ledger as JSON.
        #[arg(long)]
        json: bool,
    },
    /// CYNICpedia — registry of knowledge articles (docs, contracts, traces, designs).
    Wiki {
        #[command(subcommand)]
        command: crate::wiki_surface::WikiCommand,
    },
    /// Idea ledger: track ideas, status, consumption, and compounding.
    Idea {
        #[command(subcommand)]
        command: crate::idea_surface::IdeaCommand,
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
            let summary = match cynic_knowledge::service::summarize_index(&index) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Knowledge index error: {e}");
                    return ExitCode::FAILURE;
                }
            };

            if json {
                crate::emit_json(&summary, false)
            } else {
                println!(
                    "Knowledge Index: {} chunks indexed from docs/ and contracts/ at {}",
                    summary.chunk_count, repo_root.display()
                );
                ExitCode::SUCCESS
            }
        }
        KnowCommand::Manifest { json } => {
            let manifest = match cynic_knowledge::service::scan_corpus_manifest(&repo_root) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Knowledge manifest error: {e}");
                    return ExitCode::FAILURE;
                }
            };
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
            let graph = match cynic_knowledge::service::build_knowledge_graph(&repo_root) {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("Knowledge graph error: {e}");
                    return ExitCode::FAILURE;
                }
            };
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
        KnowCommand::Search { query, limit, commits, json } => {
            // For search, we use either the main index or a commits-indexed variant.
            let hits = if commits > 0 {
                match cynic_knowledge::service::search(
                    &cynic_knowledge::index_workspace_with_commits(&repo_root, commits),
                    &query,
                    limit,
                ) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("Knowledge search error: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                match cynic_knowledge::service::search(&index, &query, limit) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("Knowledge search error: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            };

            if json {
                let views: Vec<KnowledgeSearchHitView> = hits
                    .iter()
                    .map(|h| KnowledgeSearchHitView {
                        chunk_id: h.chunk.chunk_id.clone(),
                        source_path: h.chunk.source_path.clone(),
                        line_start: h.chunk.line_start,
                        line_end: h.chunk.line_end,
                        section: h.chunk.title(),
                        authority: format!("{:.?}", h.chunk.authority),
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
                    hits.len(), query
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
            let commits = match cynic_knowledge::service::extract_git_history(&repo_root, limit) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to extract git history: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if json {
                crate::emit_json(&commits, false)
            } else {
                println!("=== CYNIC VCS History (Last {} Commits) ===", commits.len());
                for c in &commits {
                    let short_hash = if c.hash.len() >= 8 { &c.hash[..8] } else { &c.hash };
                    println!("\n[{}] {} — {} ({})", short_hash, c.subject, c.author_name, c.timestamp);
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
        KnowCommand::Crates { layer, json } => {
            let geometry = match cynic_knowledge::service::load_crate_geometry(&repo_root) {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("Failed to load workspace geometry: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if json {
                crate::emit_json(&geometry, false)
            } else {
                println!("=== CYNIC Workspace Crate Geometry ({}) ===", geometry.schema);
                println!("  Total Crates : {}", geometry.total_crates);
                println!("  Total LOC    : {}", geometry.total_loc);

                let layers = ["L0", "L1", "L2", "L3", "L4"];
                for l in layers {
                    if let Some(ref filter_l) = layer {
                        if filter_l != l {
                            continue;
                        }
                    }
                    let in_layer = geometry.crates_in_layer(l);
                    if !in_layer.is_empty() {
                        println!("\n  [Layer {} — {} crates]:", l, in_layer.len());
                        for c in in_layer {
                            println!(
                                "    - {:<30} {:>6} LOC (fan-in: {:>2}, fan-out: {:>2})",
                                c.name, c.loc, c.fan_in(), c.fan_out()
                            );
                        }
                    }
                }
                ExitCode::SUCCESS
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
            let result = match cynic_knowledge::service::run_ask(
                &repo_root,
                &question,
                limit,
                commits,
                prompt_only,
                endpoint.clone(),
                model.clone(),
            ) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Ask error: {e}");
                    return ExitCode::FAILURE;
                }
            };

            if json {
                let out = serde_json::json!({
                    "question": question,
                    "answer": null,
                    "endpoint": result.endpoint,
                    "sources": result.sources,
                    "prompt": result.prompt
                });
                crate::emit_json(&out, false)
            } else if prompt_only {
                println!("=== CYNIC Epistemic Prompt ({}) ===", result.prompt.schema);
                println!("\n[SYSTEM INSTRUCTION]\n{}", result.prompt.system_instruction);
                println!("\n[USER PROMPT]\n{}", result.prompt.user_prompt);
                ExitCode::SUCCESS
            } else {
                // Attempt local LLM call via reqwest with streaming
                let payload = serde_json::json!({
                    "model": model,
                    "messages": [
                        {"role": "system", "content": result.prompt.system_instruction},
                        {"role": "user", "content": result.prompt.user_prompt}
                    ],
                    "temperature": 0.0,
                    "max_tokens": 600,
                    "stream": true
                });

                let is_json = json;
                let target_endpoint = result.endpoint.clone();
                let payload_thread = payload.clone();

                let (answer_text, duration_ms, error_msg) = std::thread::spawn(move || {
                    use std::io::{BufRead, BufReader, Write};

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
                            std::fs::read_to_string(cynic_types::CynicPaths::secret_path("llama-api-key",))
                                .ok()
                        })
                        .or_else(|| {
                            std::fs::read_to_string(cynic_types::CynicPaths::system_config_dir()
                                .join("llama-api-key"))
                                .ok()
                        })
                        .map(|k| k.trim().to_string());

                    let mut answer = None;
                    let mut duration = 0u128;
                    let mut err = None;

                    if let Ok(client) = client {
                        let mut req = client.post(&target_endpoint).json(&payload_thread);
                        if let Some(ref key) = api_key {
                            req = req.header("Authorization", format!("Bearer {}", key));
                        }
                        let start = std::time::Instant::now();
                        match req.send() {
                            Ok(resp) => {
                                if resp.status().is_success() {
                                    let mut accumulated = String::new();
                                    let reader = BufReader::new(resp);
                                    for line in reader.lines().map_while(Result::ok) {
                                        let trimmed = line.trim();
                                        if let Some(data) = trimmed.strip_prefix("data: ") {
                                            if data == "[DONE]" {
                                                break;
                                            }
                                            if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) {
                                                if let Some(delta) = chunk["choices"][0]["delta"].as_object() {
                                                    let tok = delta
                                                        .get("content")
                                                        .and_then(|v| v.as_str())
                                                        .or_else(|| {
                                                            delta
                                                                .get("reasoning_content")
                                                                .and_then(|v| v.as_str())
                                                        })
                                                        .unwrap_or_default();
                                                    if !tok.is_empty() {
                                                        if !is_json {
                                                            print!("{}", tok);
                                                            std::io::stdout().flush().ok();
                                                        }
                                                        accumulated.push_str(tok);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    duration = start.elapsed().as_millis();
                                    if !accumulated.is_empty() {
                                        if !is_json {
                                            println!("\n\n[Temps d'inférence : {} ms]", duration);
                                        }
                                        answer = Some(accumulated);
                                    }
                                } else {
                                    err = Some(format!("HTTP {}", resp.status().as_u16()));
                                }
                            }
                            Err(e) => {
                                err = Some(format!("Connection error: {e}"));
                            }
                        }
                    } else {
                        err = Some("Failed to construct HTTP client".into());
                    }

                    (answer, duration, err)
                })
                .join()
                .unwrap_or_else(|_| (None, 0, Some("Inference thread panicked".into())));

                if json {
                    let out = serde_json::json!({
                        "question": question,
                        "answer": answer_text,
                        "endpoint": target_endpoint,
                        "sources": result.sources,
                        "prompt": result.prompt
                    });
                    crate::emit_json(&out, false)
                } else {
                    if answer_text.is_none() {
                        let err_suffix = error_msg.map(|e| format!(" ({})", e)).unwrap_or_default();
                        println!(
                            "\n[!] Serveur d'inférence ({}) injoignable ou en erreur{}.",
                            target_endpoint, err_suffix
                        );
                        println!("    Pour exécuter avec un agent, utilisez l'option `--prompt-only`.");
                    }
                    ExitCode::SUCCESS
                }
            }
        }
        KnowCommand::Grades { file, apply, json } => {
            use cynic_eval::bench::{judge_grade_proposal, Fixture, FIXTURE_SCHEMA_V1};
            let raw = match std::fs::read_to_string(&file) {
                Ok(raw) => raw,
                Err(e) => {
                    eprintln!("cannot read {}: {e}", file.display());
                    return ExitCode::FAILURE;
                }
            };
            let proposals: Vec<cynic_eval::bench::GradeProposal> = match serde_json::from_str(&raw)
            {
                Ok(proposals) => proposals,
                Err(e) => {
                    eprintln!("invalid proposals JSON: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let fixture_path =
                repo_root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json");
            let fixture_raw = match std::fs::read_to_string(&fixture_path) {
                Ok(raw) => raw,
                Err(e) => {
                    eprintln!("cannot read {}: {e}", fixture_path.display());
                    return ExitCode::FAILURE;
                }
            };
            let mut fixture: Fixture = match serde_json::from_str(&fixture_raw) {
                Ok(fixture) => fixture,
                Err(e) => {
                    eprintln!("invalid fixture JSON: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if fixture.schema != FIXTURE_SCHEMA_V1 {
                eprintln!("fixture schema mismatch: {}", fixture.schema);
                return ExitCode::FAILURE;
            }
            let mut verdicts = Vec::new();
            let mut accepted = 0usize;
            for proposal in &proposals {
                let verdict = judge_grade_proposal(proposal, &fixture);
                if verdict.accept {
                    accepted += 1;
                    if apply {
                        // Gate passed; the duplicate check runs again inside.
                        if let Err(e) = fixture.insert_labeled(proposal) {
                            eprintln!("skip {}: {e}", proposal.id);
                            continue;
                        }
                    }
                }
                verdicts.push(verdict);
            }
            if apply && accepted > 0 {
                let out = serde_json::to_string_pretty(&fixture).unwrap_or_default();
                if let Err(e) = std::fs::write(&fixture_path, out) {
                    eprintln!("cannot write {}: {e}", fixture_path.display());
                    return ExitCode::FAILURE;
                }
            }
            if json {
                crate::emit_json(&verdicts, false)
            } else {
                for verdict in &verdicts {
                    if verdict.accept {
                        println!("[accept] {}", verdict.id);
                    } else {
                        println!("[reject] {}: {}", verdict.id, verdict.reasons.join("; "));
                    }
                }
                println!(
                    "{} proposal(s), {} accepted{}",
                    verdicts.len(),
                    accepted,
                    if apply { ", merged" } else { " (dry-run)" }
                );
                ExitCode::SUCCESS
            }
        }
        KnowCommand::Surface { out_dir, verify, max_age_hours, json } => {
            if let Some(hours) = max_age_hours {
                let published = out_dir
                    .clone()
                    .unwrap_or_else(|| repo_root.join("docs/public"))
                    .join("facts.live.json");
                return cynic_knowledge::service::check_published_surface_freshness(&published, hours)
                    .map(|age| {
                        println!("Published surface is fresh: {age}s old (limit {hours}h)");
                        ExitCode::SUCCESS
                    })
                    .unwrap_or_else(|e| {
                        eprintln!("Published surface is NOT fresh: {e}");
                        ExitCode::FAILURE
                    });
            }
            let ledger = match cynic_knowledge::service::harvest_surface_ledger(&repo_root) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Failed to harvest the facts ledger: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let verify_result = cynic_knowledge::service::surface_verify(&ledger, &repo_root, verify)
                .unwrap_or_else(|e| {
                    eprintln!("Surface verification error: {e}");
                    return ExitCode::FAILURE;
                });

            if verify {
                let mut drifted = 0usize;
                let mut unverifiable = 0usize;
                for c in &verify_result.claims_checks {
                    match &c.outcome {
                        cynic_knowledge::ClaimCheckOutcome::Match => {
                            println!(
                                "  [RECHECK OK ] {} = {}",
                                c.id, c.expected.unwrap_or_default()
                            )
                        }
                        cynic_knowledge::ClaimCheckOutcome::Mismatch { observed } => {
                            drifted += 1;
                            eprintln!(
                                "  [MISMATCH   ] {}: published {} but its command prints {}",
                                c.id, c.expected.unwrap_or_default(), observed
                            )
                        }
                        cynic_knowledge::ClaimCheckOutcome::Unverifiable(why) => {
                            unverifiable += 1;
                            println!("  [UNVERIFIED ] {}: {}", c.id, why)
                        }
                        cynic_knowledge::ClaimCheckOutcome::NoCommand => {
                            println!("  [NO COMMAND ] {}", c.id)
                        }
                    }
                }
                println!(
                    "claims re-measured: {} match, {} mismatch, {} unverified",
                    verify_result.claims_checks.len() - drifted - unverifiable,
                    drifted,
                    unverifiable
                );
                if drifted > 0 {
                    eprintln!(
                        "{drifted} claim(s) disagree with their own verify_command; surface not written"
                    );
                    return ExitCode::FAILURE;
                }
            }
            if json {
                return crate::emit_json(&ledger, false);
            }
            let out_dir = out_dir
                .unwrap_or_else(|| repo_root.join("docs/public"));
            if let Err(e) = std::fs::create_dir_all(&out_dir) {
                eprintln!("Failed to create output directory {}: {}", out_dir.display(), e);
                return ExitCode::FAILURE;
            }

            let (md, html) = cynic_knowledge::service::render_public_surface(&ledger);
            let md_path = out_dir.join("public_surface.md");
            let html_path = out_dir.join("public_surface.html");
            let json_path = out_dir.join("facts.live.json");

            if let Err(e) = std::fs::write(&md_path, &md) {
                eprintln!("Failed to write {}: {}", md_path.display(), e);
                return ExitCode::FAILURE;
            }
            if let Err(e) = std::fs::write(&html_path, &html) {
                eprintln!("Failed to write {}: {}", html_path.display(), e);
                return ExitCode::FAILURE;
            }
            if let Ok(json_str) = serde_json::to_string_pretty(&ledger) {
                let _ = std::fs::write(&json_path, json_str);
            }

            println!("=== CYNIC Epistemic Surface Generated ===");
            println!("  Markdown : {}", md_path.display());
            println!("  HTML     : {}", html_path.display());
            println!("  JSON     : {}", json_path.display());
            println!("  Repos    : {} repos across 4 eras", ledger.repos.len());
            println!(
                "  Crates   : {} (LOC: {})",
                ledger.summary_metrics.get("cynic_crates").unwrap_or(&0),
                ledger.summary_metrics.get("cynic_loc").unwrap_or(&0)
            );
            println!(
                "  Commits  : {}",
                ledger.summary_metrics.get("total_commits").unwrap_or(&0)
            );

            if verify {
                println!("\n=== Verifiable Claims Verification ===");
                for claim in &ledger.claims {
                    let status_badge = match claim.status {
                        cynic_knowledge::ClaimStatus::Demonstrated => "\x1b[32mPASS\x1b[0m",
                        cynic_knowledge::ClaimStatus::Implemented => "\x1b[34mIMPL\x1b[0m",
                        cynic_knowledge::ClaimStatus::Partial => "\x1b[33mPART\x1b[0m",
                        cynic_knowledge::ClaimStatus::Aspirational => "\x1b[35mASPI\x1b[0m",
                        cynic_knowledge::ClaimStatus::Unknown => "\x1b[33mUNKN\x1b[0m",
                    };
                    println!("  [{}] {:<28} {}", status_badge, claim.id, claim.statement);
                }
            }
            ExitCode::SUCCESS
        }
        KnowCommand::Wiki { command } => crate::wiki_surface::execute_wiki_command(command),
        KnowCommand::Idea { command } => crate::idea_surface::execute_idea(command),
    }
}

/// Resolve the cache path according to CynicPaths authority.
pub fn resolve_knowledge_cache_path() -> PathBuf {
    let system_path = cynic_types::CynicPaths::system_knowledge_cache_path();
    let system_usable = system_path
        .parent()
        .map(std::fs::create_dir_all)
        .is_some_and(|result| result.is_ok());
    if system_usable {
        system_path
    } else {
        cynic_types::CynicPaths::user_knowledge_cache_path()
    }
}

/// Find the repository root by walking up from the current directory
/// looking for a `.git` folder or `Cargo.toml`.
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
