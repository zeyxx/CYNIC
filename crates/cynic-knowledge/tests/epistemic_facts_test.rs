use cynic_knowledge::{
    harvest_live_ledger, parse_github_repos, parse_interactions_graph, render_public_surface_html,
    render_public_surface_markdown, AuthorshipNature, ClaimStatus, InteractionCategory, RepoEra,
    TechnicalDomain, EPISTEMIC_FACTS_SCHEMA_V1, INTERACTIONS_GRAPH_SCHEMA_V1,
};
use std::path::Path;

#[test]
fn github_repos_parsing_and_era_classification() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
    let json_path = repo_root.join("infra/repos-zeyxx-github.json");

    assert!(json_path.exists(), "infra/repos-zeyxx-github.json must exist");
    let content = std::fs::read_to_string(&json_path).expect("read repos json");
    let repos = parse_github_repos(&content).expect("parse repos json");

    assert_eq!(repos.len(), 35, "Expected exactly 35 GitHub repositories");

    // Era checks
    let nsi_count = repos.iter().filter(|r| r.era == RepoEra::NsiAwakening).count();
    let pentest_count = repos.iter().filter(|r| r.era == RepoEra::OffensivePentest).count();
    let solana_count = repos.iter().filter(|r| r.era == RepoEra::SolanaTrench).count();
    let rust_count = repos.iter().filter(|r| r.era == RepoEra::SovereignRust).count();

    assert!(nsi_count >= 2, "NSI awakening must have at least 2 repos");
    assert!(pentest_count >= 2, "Offensive pentest must have at least 2 repos");
    assert!(solana_count >= 10, "Solana trench must have at least 10 repos");
    assert!(rust_count >= 10, "Sovereign Rust must have at least 10 repos");

    // Individual repo sanity checks
    let cynic = repos.iter().find(|r| r.name == "CYNIC").expect("CYNIC must exist");
    assert_eq!(cynic.era, RepoEra::SovereignRust);
    assert_eq!(cynic.domain, TechnicalDomain::Systems);
    assert_eq!(cynic.authorship, AuthorshipNature::Original);

    let gasdf = repos.iter().find(|r| r.name == "GASdf").expect("GASdf must exist");
    assert_eq!(gasdf.era, RepoEra::SolanaTrench);
    assert_eq!(gasdf.domain, TechnicalDomain::Blockchain);
    assert_eq!(gasdf.authorship, AuthorshipNature::Original);

    let holdex = repos.iter().find(|r| r.name == "HolDex").expect("HolDex must exist");
    assert_eq!(holdex.authorship, AuthorshipNature::ContributedFork);
    assert!(holdex.author_commits >= 100);

    let percolator = repos.iter().find(|r| r.name == "percolator-prog").expect("percolator-prog must exist");
    assert_eq!(percolator.authorship, AuthorshipNature::UpstreamAudit);
    assert_eq!(percolator.author_commits, 0);

    let originals = repos.iter().filter(|r| r.authorship == AuthorshipNature::Original).count();
    let co_devs = repos.iter().filter(|r| r.authorship == AuthorshipNature::ContributedFork).count();
    let audits = repos.iter().filter(|r| r.authorship == AuthorshipNature::UpstreamAudit).count();

    assert_eq!(originals, 24, "Expected exactly 24 original repos");
    assert_eq!(co_devs, 3, "Expected exactly 3 co-developed forks (HolDex, ASDev, claude-mem)");
    assert_eq!(audits, 8, "Expected exactly 8 upstream audit forks");

    let nasa = repos.iter().find(|r| r.name == "NASA-Apis").expect("NASA-Apis must exist");
    assert_eq!(nasa.era, RepoEra::NsiAwakening);
}

#[test]
fn live_epistemic_ledger_harvesting_and_chunking() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();

    let ledger = harvest_live_ledger(repo_root, None);

    assert_eq!(ledger.schema, EPISTEMIC_FACTS_SCHEMA_V1);
    assert_eq!(ledger.repos.len(), 35);
    assert!(ledger.summary_metrics.get("cynic_crates").copied().unwrap_or(0) >= 40);
    assert!(ledger.summary_metrics.get("cynic_loc").copied().unwrap_or(0) > 300_000);
    assert_eq!(ledger.summary_metrics.get("git_add_all_violations").copied().unwrap_or(999), 0);

    // Verify claim statuses
    let zero_add_all = ledger.claims.iter().find(|c| c.id == "cynic.zero_git_add_all").expect("zero_git_add_all claim");
    assert_eq!(zero_add_all.status, ClaimStatus::Demonstrated);

    let crates_claim = ledger.claims.iter().find(|c| c.id == "cynic.workspace_crates").expect("workspace_crates claim");
    assert_eq!(crates_claim.status, ClaimStatus::Demonstrated);

    // Verify to_chunks produces valid DocumentChunks
    let chunks = ledger.to_chunks();
    assert!(chunks.len() >= 5, "Must produce at least 5 epistemic knowledge chunks (including interactions graph)");
    for chunk in &chunks {
        assert!(chunk.source_path.starts_with("epistemic://"));
        assert!(!chunk.content.is_empty());
        assert!(!chunk.section_hierarchy.is_empty());
    }

    let graph_chunk = chunks.iter().find(|c| c.source_path == "epistemic://author/interactions_graph").expect("interactions graph chunk");
    assert!(graph_chunk.content.contains("rust-lang/rust"));
    assert!(graph_chunk.content.contains("percolator"));
    assert!(graph_chunk.content.contains("awesome-solana-ai"));

    // Verify interactions in ledger
    assert!(ledger.interactions.is_some(), "Interactions graph must be loaded");
    let g = ledger.interactions.as_ref().unwrap();
    assert_eq!(g.total_prs, 453);
    assert_eq!(g.external_prs, 26);
    assert_eq!(g.total_issues, 8);
    assert_eq!(g.external_issues, 5);

    // Verify rendering surfaces
    let md = render_public_surface_markdown(&ledger);
    assert!(md.contains("Titouan Corneloup"));
    assert!(md.contains("Live Substrate Receipts"));
    assert!(md.contains("The 4-Era GitHub Odyssey"));
    assert!(md.contains("GitHub Interactions & Ecosystem Graph"));
    assert!(md.contains("Security Bounties"));
    assert!(md.contains("CYNIC"));

    let html = render_public_surface_html(&ledger);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("Titouan Corneloup"));
    assert!(html.contains("CYNIC Epistemic Engine"));
    assert!(html.contains("badge-demonstrated"));
    assert!(html.contains("badge-bounty"));
    assert!(html.contains("badge-compiler"));
}

#[test]
fn github_interactions_graph_parsing_and_categorization() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
    let json_path = repo_root.join("infra/github-interactions-graph.json");

    assert!(json_path.exists(), "infra/github-interactions-graph.json must exist");
    let content = std::fs::read_to_string(&json_path).expect("read interactions json");
    let graph = parse_interactions_graph(&content).expect("parse interactions graph");

    assert_eq!(graph.schema, INTERACTIONS_GRAPH_SCHEMA_V1);
    assert_eq!(graph.total_prs, 453);
    assert_eq!(graph.external_prs, 26);
    assert_eq!(graph.internal_prs, 427);
    assert_eq!(graph.total_issues, 8);
    assert_eq!(graph.external_issues, 5);
    assert_eq!(graph.internal_issues, 3);
    assert_eq!(graph.external_ecosystems.len(), 9);

    // Verify specific categorized interactions
    let rust_bug = graph.items.iter().find(|i| i.repository == "rust-lang/rust" && i.number == 156563).expect("rust bug");
    assert_eq!(rust_bug.category, InteractionCategory::CompilerBug);
    assert!(!rust_bug.is_pr);
    assert!(rust_bug.impact_note.as_ref().unwrap().contains("LLVM FastISel"));

    let anatoly_bounty = graph.items.iter().find(|i| i.repository == "aeyakovenko/percolator-prog" && i.number == 110).expect("percolator bounty");
    assert_eq!(anatoly_bounty.category, InteractionCategory::SecurityBounty);
    assert!(anatoly_bounty.impact_note.as_ref().unwrap().contains("100,000,000x Price Manipulation"));

    let solana_ai = graph.items.iter().find(|i| i.repository == "solana-foundation/awesome-solana-ai" && i.number == 13).expect("solana ai PR");
    assert_eq!(solana_ai.category, InteractionCategory::ProtocolContribution);
    assert_eq!(solana_ai.state, "merged");
    assert!(solana_ai.is_pr);

    let holdex_ci = graph.items.iter().find(|i| i.repository == "sollama58/HolDex" && i.number == 6).expect("holdex PR");
    assert_eq!(holdex_ci.category, InteractionCategory::PartnerFeature);
    assert_eq!(holdex_ci.state, "merged");
}
