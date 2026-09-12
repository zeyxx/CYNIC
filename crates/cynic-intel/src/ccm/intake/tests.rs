//! Tests for CCM intake hashing, semantic slugs, and observation building.

use super::*;

#[test]
fn slug_dev_conventional_commit_with_scope() {
    assert_eq!(
        semantic_slug("dev", "abc1234: feat(inference): add new feature"),
        "dev:feat:inference"
    );
}

#[test]
fn slug_dev_conventional_commit_no_scope() {
    let slug = semantic_slug("dev", "fix: handle edge case");
    assert!(slug.starts_with("dev:fix"), "got: {slug}");
}

#[test]
fn slug_dev_with_file_reference() {
    assert_eq!(
        semantic_slug("dev", "refactor: split storage.rs into modules"),
        "dev:refactor:storage"
    );
}

#[test]
fn slug_dev_similar_commits_coalesce() {
    let a = semantic_slug("dev", "abc1234: feat(inference): add timeout");
    let b = semantic_slug("dev", "def5678: feat(inference): improve retry");
    assert_eq!(
        a, b,
        "similar feat(inference) commits should produce same slug"
    );
}

#[test]
fn slug_dev_different_types_differ() {
    let a = semantic_slug("dev", "feat: add feature");
    let b = semantic_slug("dev", "fix: fix bug");
    assert_ne!(a, b);
}

#[test]
fn slug_token_extracts_profile_from_enriched_stimulus() {
    let content = "[DOMAIN: token-analysis]\n\n[METRICS]\nmint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v\nname: USD Coin\nmint_authority: ACTIVE (can mint more tokens)\nfreeze_authority: ACTIVE (can freeze wallets)\nmetadata_mutable: YES";
    let slug = semantic_slug("token-analysis", content);
    assert_eq!(slug, "token-analysis:active:active:mutable", "got: {slug}");
}

#[test]
fn slug_token_revoked_profile() {
    let content = "mint_authority: REVOKED (supply is fixed)\nfreeze_authority: REVOKED\nmetadata_mutable: NO";
    let slug = semantic_slug("token", content);
    assert_eq!(slug, "token:revoked:revoked:immutable", "got: {slug}");
}

#[test]
fn slug_token_analysis_same_profile_different_tokens_coalesce() {
    let usdc = "[METRICS]\nmint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v\nname: USD Coin\nmint_authority: ACTIVE\nfreeze_authority: ACTIVE\nmetadata_mutable: YES";
    let other = "[METRICS]\nmint: 7kbnvuGBxxj8AG9qp8Scn56muWGaRaFqxg1FsRp3PaFT\nname: UXD Stablecoin\nmint_authority: ACTIVE\nfreeze_authority: ACTIVE\nmetadata_mutable: YES";
    let a = semantic_slug("token-analysis", usdc);
    let b = semantic_slug("token-analysis", other);
    assert_eq!(a, b, "tokens with same authority profile should coalesce");
}

#[test]
fn slug_token_different_profiles_differ() {
    let active = "mint_authority: ACTIVE\nfreeze_authority: ACTIVE\nmetadata_mutable: YES";
    let revoked = "mint_authority: REVOKED\nfreeze_authority: REVOKED\nmetadata_mutable: NO";
    let a = semantic_slug("token", active);
    let b = semantic_slug("token", revoked);
    assert_ne!(
        a, b,
        "different authority profiles should produce different slugs"
    );
}

#[test]
fn slug_token_analysis_enriched_stimulus_coalesces() {
    let stim_a = "[METRICS]\nmint: 9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump\nname: asdf\nmint_authority: REVOKED\nfreeze_authority: REVOKED\nmetadata_mutable: YES\nprice: $0.001";
    let stim_b = "[METRICS]\nmint: 9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump\nname: asdf\nmint_authority: REVOKED\nfreeze_authority: REVOKED\nmetadata_mutable: YES\nprice: $0.002";
    let a = semantic_slug("token-analysis", stim_a);
    let b = semantic_slug("token-analysis", stim_b);
    assert_eq!(a, b, "same profile with different prices should coalesce");
}

#[test]
fn slug_token_unstructured_falls_back_to_words() {
    let content = "POCKS token looks suspicious";
    let slug = semantic_slug("token", content);
    assert!(slug.starts_with("token:"), "got: {slug}");
    assert!(
        slug.contains("pocks"),
        "should fall back to first words, got: {slug}"
    );
}

#[test]
fn slug_token_partial_fields_still_profile() {
    let content = "mint_authority: REVOKED\nname: Jupiter";
    let slug = semantic_slug("token", content);
    assert_eq!(slug, "token:revoked:unk:unk", "got: {slug}");
}

#[test]
fn slug_session_claude() {
    assert_eq!(
        semantic_slug("session", "[claude-abc123] Edit: modified storage.rs"),
        "session:agent:claude"
    );
}

#[test]
fn slug_session_gemini() {
    assert_eq!(
        semantic_slug("session", "[gemini-xyz] Read: checked file"),
        "session:agent:gemini"
    );
}

#[test]
fn slug_d2_model_and_engine() {
    let slug = semantic_slug("D2", "Qwen 3.6 27B running on llama.cpp with MTP enabled");
    assert_eq!(slug, "D2:qwen:llamacpp");
}

#[test]
fn slug_d2_model_and_topic() {
    let slug = semantic_slug("D2", "Qwen3.6 27B quantization comparison IQ3 vs Q4_K");
    assert_eq!(slug, "D2:qwen:quant");
}

#[test]
fn slug_d2_engine_and_topic() {
    let slug = semantic_slug(
        "D2",
        "vLLM benchmark: 1200 tok/s on H100 with batched inference",
    );
    assert_eq!(slug, "D2:vllm:bench");
}

#[test]
fn slug_d2_speculative_decoding() {
    let slug = semantic_slug("D2", "MTP speculative decoding gives 1.85x on RTX 3090");
    assert_eq!(slug, "D2:spec");
}

#[test]
fn slug_d2_same_model_engine_coalesces() {
    let a = semantic_slug("D2", "Qwen 3.6 on llama.cpp gets 19.7 tok/s");
    let b = semantic_slug("D2", "Qwen 3.5 using llama-server with GGUF Q4");
    assert_eq!(a, b, "same model+engine cluster should coalesce");
}

#[test]
fn slug_d2_different_models_differ() {
    let a = semantic_slug("D2", "Qwen benchmark results on H100");
    let b = semantic_slug("D2", "Llama benchmark results on H100");
    assert_ne!(a, b);
}

#[test]
fn slug_chess_preserved() {
    assert_eq!(semantic_slug("chess", "1. e4 c5"), "chess:1. e4 c5");
}

#[test]
fn slug_generic_first_words() {
    let slug = semantic_slug("claim", "The earth is flat and vaccines cause autism");
    assert!(slug.starts_with("claim:"), "got: {slug}");
    assert!(slug.contains("earth"), "got: {slug}");
    assert!(slug.contains("flat"), "got: {slug}");
}

#[test]
fn slug_general_bash_observations_coalesce() {
    let a = semantic_slug("general", "[claude-abc123] Bash rtk cargo build --release");
    let b = semantic_slug(
        "general",
        "[claude-def456] Bash curl -s http://localhost:3030/health",
    );
    let c = semantic_slug(
        "general",
        "[claude-abc123] Bash sleep 3 && systemctl status",
    );
    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(a, "general:tool:bash");
}

#[test]
fn slug_general_write_observations_coalesce() {
    let a = semantic_slug("general", "[claude-abc123] Write src/main.rs");
    let b = semantic_slug("general", "[claude-xyz789] Write /tmp/test.txt");
    assert_eq!(a, b);
    assert_eq!(a, "general:tool:write");
}

#[test]
fn slug_general_different_tools_differ() {
    let bash = semantic_slug("general", "[claude-abc] Bash echo hello");
    let write = semantic_slug("general", "[claude-abc] Write /tmp/file");
    let read = semantic_slug("general", "[claude-abc] Read /tmp/file");
    assert_ne!(bash, write);
    assert_ne!(write, read);
}

#[test]
fn slug_general_kernel_self_obs_coalesce() {
    let a = semantic_slug("general", "[kernel] boot self: version=0.7.7 dogs=4");
    let b = semantic_slug("general", "[kernel] boot self: version=0.7.8 dogs=5");
    assert_eq!(a, b);
    assert_eq!(a, "general:tool:boot");
}

#[test]
fn slug_general_session_compliance_coalesces() {
    let a = semantic_slug(
        "general",
        "[claude-aaa] session_compliance session_end: score=0.54",
    );
    let b = semantic_slug(
        "general",
        "[claude-bbb] session_compliance session_end: score=0.88",
    );
    assert_eq!(a, b);
    assert_eq!(a, "general:tool:session_compliance");
}

#[test]
fn slug_general_non_tool_falls_back_to_words() {
    let slug = semantic_slug("general", "The quick brown fox jumps over the lazy dog");
    assert!(slug.starts_with("general:"), "got: {slug}");
    assert!(slug.contains("quick"), "got: {slug}");
}

#[test]
fn infer_domain_rust() {
    assert_eq!(infer_domain(Some("src/judge.rs"), None), "rust");
}

#[test]
fn infer_domain_typescript() {
    assert_eq!(infer_domain(Some("App.tsx"), None), "typescript");
    assert_eq!(infer_domain(Some("utils.ts"), None), "typescript");
}

#[test]
fn infer_domain_javascript() {
    assert_eq!(infer_domain(Some("index.js"), None), "javascript");
}

#[test]
fn infer_domain_python() {
    assert_eq!(infer_domain(Some("train.py"), None), "python");
}

#[test]
fn infer_domain_config() {
    assert_eq!(infer_domain(Some("Cargo.toml"), None), "config");
    assert_eq!(infer_domain(Some("package.json"), None), "config");
    assert_eq!(infer_domain(Some("config.yaml"), None), "config");
}

#[test]
fn infer_domain_docs() {
    assert_eq!(infer_domain(Some("README.md"), None), "docs");
}

#[test]
fn infer_domain_general_fallback() {
    assert_eq!(infer_domain(Some("binary.wasm"), None), "general");
    assert_eq!(infer_domain(None, None), "general");
    assert_eq!(infer_domain(Some("Makefile"), None), "general");
}

#[test]
fn infer_domain_infra_service() {
    assert_eq!(infer_domain(Some("cynic-kernel.service"), None), "infra");
    assert_eq!(infer_domain(Some("backup.timer"), None), "infra");
}

#[test]
fn infer_domain_infra_targets() {
    assert_eq!(infer_domain(Some("cynic-host-proxmox"), None), "infra");
    assert_eq!(infer_domain(Some("cynic-host-gpu"), None), "infra");
    assert_eq!(infer_domain(Some("llama-server"), None), "infra");
    assert_eq!(infer_domain(Some("surrealdb"), None), "infra");
}

#[test]
fn infer_domain_self_probe_tool() {
    assert_eq!(infer_domain(Some("localhost"), Some("self-probe")), "infra");
    assert_eq!(infer_domain(None, Some("self-probe")), "infra");
}

#[test]
fn normalize_collapses_whitespace_and_lowercases() {
    assert_eq!(
        normalize_for_hash("  Chess:  1. E4  c5  "),
        "chess: 1. e4 c5"
    );
}

#[test]
fn normalize_identical_content_hashes_same() {
    let a = content_hash(&normalize_for_hash("chess:1. e4 c5 — The Sicilian Defense"));
    let b = content_hash(&normalize_for_hash(
        "chess:1. e4 c5 —  The  Sicilian  Defense",
    ));
    let c = content_hash(&normalize_for_hash("Chess:1. E4 C5 — The Sicilian Defense"));
    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn normalize_different_content_hashes_differ() {
    let a = content_hash(&normalize_for_hash("chess:1. e4 c5"));
    let b = content_hash(&normalize_for_hash("chess:1. d4 d5"));
    assert_ne!(a, b);
}

#[test]
fn slug_trading_extracts_side_and_symbol() {
    let slug = semantic_slug("trading", "LONG 2.0x on SOL @ 142.50");
    assert_eq!(slug, "trading:long:sol");
}

#[test]
fn slug_trading_same_side_and_symbol_coalesces() {
    let a = semantic_slug("trading", "LONG 2.0x on SOL @ 142.50");
    let b = semantic_slug("trading", "LONG 1x on SOL @ 145.75");
    let c = semantic_slug("trading", "LONG 5.0x on SOL @ 140.00");
    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn slug_trading_different_side_differs() {
    let long = semantic_slug("trading", "LONG 2.0x on SOL @ 142.50");
    let short = semantic_slug("trading", "SHORT 2.0x on SOL @ 142.50");
    assert_ne!(long, short);
}

#[test]
fn slug_trading_different_symbol_differs() {
    let sol = semantic_slug("trading", "LONG 2.0x on SOL @ 142.50");
    let btc = semantic_slug("trading", "LONG 2.0x on BTC @ 98500.00");
    assert_ne!(sol, btc);
}

#[test]
fn slug_trading_fallback_excludes_side() {
    let slug = semantic_slug(
        "trading",
        "LONG SOL at $148, leverage 5x, based on breakout above resistance",
    );
    assert_eq!(slug, "trading:long:sol");
}
