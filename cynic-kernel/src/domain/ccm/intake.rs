//! CCM Intake — content hashing, pattern extraction, observation building.
//!
//! Ingestion side of the CCM pipeline: normalize content, extract patterns
//! from observations, infer domains, build observation records.

// ── CONTENT HASHING ─────────────────────────────────────────
/// Deterministic content hash for crystal IDs. FNV-1a — not cryptographic,
/// just stable content addressing for deduplication. Changing this algo
/// would orphan all existing crystals in the DB.
pub fn content_hash(input: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    h
}

/// Normalize content before FNV hashing to reduce fragmentation.
/// Lowercase + collapse whitespace + trim. Two stimuli that differ only
/// in casing or spacing will hash to the same crystal ID.
/// Used as fallback when embedding-based KNN merge is unavailable.
pub fn normalize_for_hash(input: &str) -> String {
    input
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ── SEMANTIC SLUG ─────────────────────────────────────────
/// Reduce content to a domain-aware "semantic slug" before FNV hashing.
/// Without this, every unique stimulus creates a new crystal — chess works
/// because "1. e4 c5" repeats exactly, but dev/token/session content never does.
///
/// The slug captures the TOPIC (what category of thing), not the INSTANCE
/// (what specific thing). This lets observations accumulate on the same crystal
/// even when the exact text varies.
///
/// Returns `"domain:slug"` ready for `normalize_for_hash` → `content_hash`.
pub fn semantic_slug(domain: &str, content: &str) -> String {
    let slug = match domain {
        "dev" => slug_dev(content),
        "token" => slug_token(content),
        "session" => slug_session(content),
        "trading" => slug_trading(content),
        "chess" => return format!("chess:{content}"), // chess: keep exact (already repetitive)
        _ => first_n_words(content, 3),
    };
    format!("{domain}:{slug}")
}

/// Dev commits: extract conventional commit type + scope.
/// "abc1234: feat(inference): add feature" → "feat:inference"
/// "fix: handle edge case in storage" → "fix:storage" (first .rs file or first noun)
fn slug_dev(content: &str) -> String {
    // Strip leading git hash: 7+ hex chars followed by separator (: or space).
    // Can't use trim_start_matches(hex) — it eats "f" from "fix" since 'f' is hex.
    let trimmed = content.trim();
    let hex_prefix_len = trimmed
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .count();
    let stripped = if hex_prefix_len >= 7 {
        trimmed[hex_prefix_len..]
            .trim_start_matches(':')
            .trim_start_matches(' ')
            .trim()
    } else {
        trimmed
    };

    // Parse conventional commit: type(scope): message
    let types = [
        "feat", "fix", "refactor", "chore", "docs", "test", "style", "perf", "ci", "build",
    ];

    let lower = stripped.to_lowercase();
    for t in types {
        if lower.starts_with(t) {
            let rest = &stripped[t.len()..];
            // Check for (scope)
            if let Some(rest_after_paren) = rest.strip_prefix('(')
                && let Some(end) = rest_after_paren.find(')')
            {
                let scope = &rest_after_paren[..end];
                return format!("{t}:{scope}").to_lowercase();
            }
            // No explicit scope — extract first significant file reference
            if let Some(file) = extract_file_stem(rest) {
                return format!("{t}:{file}").to_lowercase();
            }
            return t.to_string();
        }
    }

    // No conventional commit type — use first 3 words
    first_n_words(stripped, 3)
}

/// Token analysis: extract mint address (base58, 32-44 chars) or token name.
fn slug_token(content: &str) -> String {
    // Look for a Solana address (base58: 32-44 chars of alphanumeric, no 0/O/I/l)
    for word in content.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
        if clean.len() >= 32
            && clean.len() <= 44
            && clean
                .chars()
                .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
        {
            return clean.to_string();
        }
    }
    // No address found — use first 3 words
    first_n_words(content, 3)
}

/// Session: collapse to agent type only.
fn slug_session(content: &str) -> String {
    // "[agent_id] tool: context" → extract agent type
    let lower = content.to_lowercase();
    if lower.contains("claude") {
        "agent:claude".to_string()
    } else if lower.contains("gemini") {
        "agent:gemini".to_string()
    } else if lower.contains("hermes") {
        "agent:hermes".to_string()
    } else {
        "summary".to_string()
    }
}

/// Trading: extract side + symbol only.
/// "LONG 2.0x on SOL @ 142.50" → "long:sol"
/// "SHORT 1x on BTC @ 98500" → "short:btc"
/// Ignores leverage, entry price, and timestamp so observations accumulate across different entries.
fn slug_trading(content: &str) -> String {
    let lower = content.to_lowercase();

    // Extract side (long, short, buy, sell)
    let side = if lower.contains("long") {
        "long"
    } else if lower.contains("short") {
        "short"
    } else if lower.contains("buy") {
        "buy"
    } else if lower.contains("sell") {
        "sell"
    } else {
        // Fallback: first word as side
        lower.split_whitespace().next().unwrap_or("unknown")
    };

    // Extract symbol: look for capitalized ticker after "on" or as all-caps word
    let symbol = lower
        .split_whitespace()
        .skip_while(|w| w != &"on")
        .nth(1)
        .and_then(|w| {
            let clean = w.trim_matches(|c: char| !c.is_alphabetic());
            if clean.len() >= 2 && clean.len() <= 6 {
                Some(clean.to_string())
            } else {
                None
            }
        })
        .or_else(|| {
            // Fallback: look for any 2-4 letter word that isn't the side itself
            lower
                .split_whitespace()
                .find(|w| {
                    let clean = w.trim_matches(|c: char| !c.is_alphabetic());
                    clean.len() >= 2 && clean.len() <= 4 && clean != side
                })
                .map(|w| w.trim_matches(|c: char| !c.is_alphabetic()).to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    format!("{side}:{symbol}")
}

/// Generic: first N significant words, lowercased.
fn first_n_words(content: &str, n: usize) -> String {
    content
        .split_whitespace()
        .filter(|w| w.len() > 2) // skip short words
        .take(n)
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase()
}

/// Extract a Rust/source file stem from text. "in storage.rs" → "storage"
fn extract_file_stem(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_');
        if clean.contains('.') {
            let stem = clean.split('.').next().unwrap_or(clean);
            if !stem.is_empty() && stem.len() > 1 {
                return Some(stem.to_lowercase());
            }
        }
    }
    None
}

// ── WORKFLOW AGGREGATOR ────────────────────────────────────
/// Aggregate raw observations into CCM crystals. Runs periodically.
/// Extracts frequency patterns and co-occurrences from the observation table,
/// then feeds them into observe_crystal as workflow domain crystals.
///
/// Pure logic: takes query results, returns (id, content, domain, score) tuples.
/// Caller is responsible for DB queries and observe_crystal calls.
pub fn extract_patterns(
    rows: &[crate::domain::storage::ObservationFrequency],
    total_observations: u64,
) -> Vec<(String, String, f64)> {
    if total_observations == 0 {
        return Vec::new();
    }

    let mut patterns: Vec<(String, String, f64)> = Vec::new();

    for row in rows {
        if row.target.is_empty() || row.freq < 3 {
            continue;
        }

        let score = row.freq as f64 / total_observations as f64;
        let id = format!(
            "wf_{:x}",
            content_hash(&format!("{}:{}", row.tool, row.target))
        );
        let content = format!("{} {} — {}x observed", row.tool, row.target, row.freq);

        patterns.push((id, content, score));
    }

    patterns
}

/// Extract co-occurrence patterns from session-grouped observations.
/// Input: rows of {session_id, target} sorted by session_id.
/// Output: (crystal_id, content, score) for pairs that co-occur in 2+ sessions.
///
/// Pure function — all co-occurrence computation happens in Rust, not SQL.
pub fn extract_cooccurrences(
    rows: &[crate::domain::storage::SessionTarget],
) -> Vec<(String, String, f64)> {
    use std::collections::{HashMap, HashSet};

    let mut sessions: HashMap<String, HashSet<String>> = HashMap::new();
    for row in rows {
        if row.session_id.is_empty() || row.target.is_empty() {
            continue;
        }
        sessions
            .entry(row.session_id.clone())
            .or_default()
            .insert(row.target.clone());
    }

    let multi_target_sessions: Vec<&HashSet<String>> = sessions
        .values()
        .filter(|targets| targets.len() >= 2)
        .collect();

    if multi_target_sessions.is_empty() {
        return Vec::new();
    }

    let total_sessions = multi_target_sessions.len() as f64;

    let mut pair_counts: HashMap<(String, String), u32> = HashMap::new();
    for targets in &multi_target_sessions {
        let mut sorted: Vec<&String> = targets.iter().collect();
        sorted.sort();
        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                let key = (sorted[i].clone(), sorted[j].clone());
                *pair_counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    let mut patterns: Vec<(String, String, f64)> = pair_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|((a, b), count)| {
            let score = count as f64 / total_sessions;
            let short_a = a.rsplit('/').next().unwrap_or(&a);
            let short_b = b.rsplit('/').next().unwrap_or(&b);
            let id = format!("co_{:x}", content_hash(&format!("{a}:{b}")));
            let content = format!(
                "{} + {} — co-edited in {}% of sessions",
                short_a,
                short_b,
                (score * 100.0) as u32
            );
            (id, content, score)
        })
        .collect();

    patterns.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    patterns.truncate(20);
    patterns
}

// ── DOMAIN INFERENCE ────────────────────────────────────────
/// Infer domain from file extension or target pattern.
/// Moved from api/rest/observe.rs — this is domain logic (K5 compliance).
pub fn infer_domain(target: Option<&str>, tool: Option<&str>) -> String {
    if let Some(t) = tool
        && t == "self-probe"
    {
        return "infra".to_string();
    }

    let target = match target {
        Some(t) if !t.is_empty() => t,
        _ => return "general".to_string(),
    };

    if target.starts_with("cynic-")
        || target.contains("llama-server")
        || target.contains("surrealdb")
    {
        return "infra".to_string();
    }

    target
        .rsplit('.')
        .next()
        .map(|ext| match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "md" => "docs",
            "toml" | "json" | "yaml" | "yml" => "config",
            "service" | "timer" => "infra",
            _ => "general",
        })
        .unwrap_or("general")
        .to_string()
}

// ── OBSERVATION BUILDER ─────────────────────────────────────
/// Build an `Observation` from raw API parameters.
///
/// Single source of truth for the REST and MCP observe surfaces (K3/K13).
/// Both `POST /observe` and `cynic_observe` call this — zero duplication.
#[allow(clippy::too_many_arguments)]
// WHY: K13 — single shared function for REST+MCP observe surfaces. All 8 fields
// map directly to Observation struct fields; grouping into a sub-struct would
// add indirection without reducing complexity.
pub fn build_observation(
    tool: String,
    target: Option<String>,
    domain: Option<String>,
    status: Option<String>,
    context: Option<String>,
    project: Option<String>,
    agent_id: Option<String>,
    session_id: Option<String>,
    tags: Option<Vec<String>>,
) -> crate::domain::storage::Observation {
    let resolved_domain = domain.unwrap_or_else(|| infer_domain(target.as_deref(), Some(&tool)));

    crate::domain::storage::Observation {
        project: project.unwrap_or_else(|| "CYNIC".into()),
        agent_id: agent_id.unwrap_or_else(|| "unknown".into()),
        tool,
        target: crate::domain::sanitize::sanitize_observation_target(&target.unwrap_or_default()),
        domain: resolved_domain,
        status: status.unwrap_or_else(|| "success".into()),
        context: context
            .map(|c| c.chars().take(200).collect())
            .unwrap_or_default(),
        session_id: session_id.unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        tags: tags.unwrap_or_default(),
    }
}

// ── TESTS ───────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_patterns_basic() {
        use crate::domain::storage::ObservationFrequency;
        let rows = vec![
            ObservationFrequency {
                target: "storage.rs".into(),
                tool: "Edit".into(),
                freq: 10,
            },
            ObservationFrequency {
                target: "judge.rs".into(),
                tool: "Edit".into(),
                freq: 5,
            },
            ObservationFrequency {
                target: "ls".into(),
                tool: "Bash".into(),
                freq: 2,
            },
        ];
        let patterns = extract_patterns(&rows, 20);
        assert_eq!(patterns.len(), 2);
        assert!((patterns[0].2 - 0.5).abs() < 1e-10);
        assert!((patterns[1].2 - 0.25).abs() < 1e-10);
        assert!(patterns[0].1.contains("storage.rs"));
    }

    #[test]
    fn extract_patterns_empty() {
        let patterns = extract_patterns(&[], 0);
        assert!(patterns.is_empty());
    }

    #[test]
    fn extract_patterns_skips_empty_targets() {
        use crate::domain::storage::ObservationFrequency;
        let rows = vec![ObservationFrequency {
            target: "".into(),
            tool: "Bash".into(),
            freq: 10,
        }];
        let patterns = extract_patterns(&rows, 10);
        assert!(patterns.is_empty());
    }

    #[test]
    fn cooccurrence_basic() {
        use crate::domain::storage::SessionTarget;
        let rows = vec![
            SessionTarget {
                session_id: "s1".into(),
                target: "/src/a.rs".into(),
            },
            SessionTarget {
                session_id: "s1".into(),
                target: "/src/b.rs".into(),
            },
            SessionTarget {
                session_id: "s2".into(),
                target: "/src/a.rs".into(),
            },
            SessionTarget {
                session_id: "s2".into(),
                target: "/src/b.rs".into(),
            },
            SessionTarget {
                session_id: "s3".into(),
                target: "/src/a.rs".into(),
            },
        ];
        let patterns = extract_cooccurrences(&rows);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].1.contains("a.rs"));
        assert!(patterns[0].1.contains("b.rs"));
        assert!((patterns[0].2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cooccurrence_filters_single_occurrence() {
        use crate::domain::storage::SessionTarget;
        let rows = vec![
            SessionTarget {
                session_id: "s1".into(),
                target: "/src/x.rs".into(),
            },
            SessionTarget {
                session_id: "s1".into(),
                target: "/src/y.rs".into(),
            },
        ];
        let patterns = extract_cooccurrences(&rows);
        assert!(patterns.is_empty());
    }

    #[test]
    fn cooccurrence_empty_sessions() {
        let patterns = extract_cooccurrences(&[]);
        assert!(patterns.is_empty());
    }

    #[test]
    fn cooccurrence_skips_empty_session_id() {
        use crate::domain::storage::SessionTarget;
        let rows = vec![
            SessionTarget {
                session_id: "".into(),
                target: "/src/a.rs".into(),
            },
            SessionTarget {
                session_id: "".into(),
                target: "/src/b.rs".into(),
            },
        ];
        let patterns = extract_cooccurrences(&rows);
        assert!(patterns.is_empty());
    }

    // ── Semantic slug tests ─────────────────────────────────

    #[test]
    fn slug_dev_conventional_commit_with_scope() {
        assert_eq!(
            semantic_slug("dev", "abc1234: feat(inference): add new feature"),
            "dev:feat:inference"
        );
    }

    #[test]
    fn slug_dev_conventional_commit_no_scope() {
        // "fix: handle edge case" — no scope, no file reference → just the type
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
    fn slug_token_extracts_address() {
        let content = "Token analysis: 9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump has 20 holders";
        let slug = semantic_slug("token", content);
        assert!(slug.contains("9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump"));
    }

    #[test]
    fn slug_token_same_mint_coalesces() {
        let a = semantic_slug(
            "token",
            "Token 9zB5wRarXMj has 20 holders, 98% concentration",
        );
        let b = semantic_slug(
            "token",
            "Token 9zB5wRarXMj has 25 holders, 95% concentration",
        );
        assert_eq!(a, b, "same token name should produce same slug");
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
    fn slug_chess_preserved() {
        assert_eq!(semantic_slug("chess", "1. e4 c5"), "chess:1. e4 c5");
    }

    #[test]
    fn slug_generic_first_words() {
        let slug = semantic_slug("claim", "The earth is flat and vaccines cause autism");
        // first_n_words skips words <=2 chars ("The", "is", "and")
        assert!(slug.starts_with("claim:"), "got: {slug}");
        assert!(
            slug.contains("earth"),
            "should contain 'earth', got: {slug}"
        );
        assert!(slug.contains("flat"), "should contain 'flat', got: {slug}");
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
        assert_eq!(infer_domain(Some("cynic-core"), None), "infra");
        assert_eq!(infer_domain(Some("cynic-gpu"), None), "infra");
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
        assert_eq!(
            a, b,
            "same side/symbol should coalesce despite different leverage"
        );
        assert_eq!(
            b, c,
            "observations should accumulate across different prices"
        );
    }

    #[test]
    fn slug_trading_different_side_differs() {
        let long = semantic_slug("trading", "LONG 2.0x on SOL @ 142.50");
        let short = semantic_slug("trading", "SHORT 2.0x on SOL @ 142.50");
        assert_ne!(
            long, short,
            "different sides should produce different slugs"
        );
    }

    #[test]
    fn slug_trading_different_symbol_differs() {
        let sol = semantic_slug("trading", "LONG 2.0x on SOL @ 142.50");
        let btc = semantic_slug("trading", "LONG 2.0x on BTC @ 98500.00");
        assert_ne!(sol, btc, "different symbols should produce different slugs");
    }

    #[test]
    fn slug_trading_fallback_excludes_side() {
        // When "on <SYMBOL>" pattern fails (word after "on" too long), fallback
        // must not pick the side itself as the symbol.
        let slug = semantic_slug(
            "trading",
            "LONG SOL at $148, leverage 5x, based on breakout above resistance",
        );
        assert_eq!(
            slug, "trading:long:sol",
            "fallback should pick 'sol' not 'long' as symbol"
        );
    }
}
