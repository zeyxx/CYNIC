//! Typed error-bucket signature table — single canonical detection source.
//!
//! Replaces loose substring matching (structural invariant 6: one decision
//! source). The previous table (`default_bucket_patterns`) matched raw
//! substrings like `"ice"` / `"oom"` and produced ~99.8% false positives on
//! the real corpus (93 461 substring hits vs 188 word-bounded ICE-family
//! hits; `service`/`device` alone explained 87 455 lines).
//!
//! Every pattern is a compiled, word-anchored regex. Buckets are ordered:
//! consumers that need first-match semantics keep it, consumers that need
//! per-line multi-bucket counting iterate all entries.

use std::sync::OnceLock;

use regex::{Regex, RegexBuilder};

/// One detection rule: bucket label + anchored pattern.
pub struct BucketSignature {
    pub domain: &'static str,
    pub pattern: &'static str,
}

/// Canonical table (ordered; first match per bucket = priority order).
const SIGNATURES: &[BucketSignature] = &[
    // toolchain_ice: real compiler crashes / toolchain failures only.
    // `rustc`/`toolchain` bare mentions are NOT events (2026-08-25 fix:
    // mention-vs-event trap — the same class of defect as the old loose
    // "ice" substring). A failure verb or crash noun must co-occur.
    BucketSignature {
        domain: "toolchain_ice",
        pattern: r"\b(?:SIGSEGV|segfault|internal\s+compiler\s+error)\b|\bICE\b",
    },
    BucketSignature {
        domain: "toolchain_ice",
        pattern: r"\brustc\b[^\n]{0,120}?\b(?:SIGSEGV|segfault|internal\s+compiler\s+error|panicked|crashed|interrupted)\b|\b(?:SIGSEGV|segfault|internal\s+compiler\s+error|panicked|crashed|interrupted)\b[^\n]{0,120}?\brustc\b",
    },
    BucketSignature {
        domain: "toolchain_ice",
        pattern: r"\bcargo\s+(?:check|build|test)\b[^\n]{0,80}?\b(?:error|failed|SIGSEGV)\b",
    },
    BucketSignature {
        domain: "toolchain_ice",
        pattern: r"\btoolchain\b[^\n]{0,60}?\b(?:failed|failure|error|broken|missing|unavailable)\b|\b(?:failed|failure|broken|missing)\b[^\n]{0,60}?\btoolchain\b",
    },
    // cargo_build: explicit build failure outcomes (not mere mentions).
    BucketSignature {
        domain: "cargo_build",
        pattern: r"\b(?:could\s+not\s+compile|compilation\s+failed|error\[E\d{4}\])\b",
    },
    // network: transport failures.
    BucketSignature {
        domain: "network",
        pattern: r"\b(?:timed?\s*out|timeout|connection\s+(?:refused|reset|closed)|DNS\s+(?:failure|error)|no\s+route\s+to\s+host|connection\s+attempt\s+failed)\b",
    },
    // oom: memory exhaustion.
    BucketSignature {
        domain: "oom",
        pattern: r"\b(?:OOMKilled|out\s+of\s+memory|cannot\s+allocate\s+memory)\b",
    },
    // permission: authorization failures.
    BucketSignature {
        domain: "permission",
        pattern: r"\b(?:permission\s+denied|operation\s+not\s+permitted|EACCES\b|access\s+denied)\b",
    },
    // notfound: missing artifacts/commands.
    BucketSignature {
        domain: "notfound",
        pattern: r"\b(?:command\s+not\s+found|no\s+such\s+file\s+or\s+directory|undefined\s+reference\s+to|cannot\s+find\s+(?:value|crate|struct|function))\b",
    },
];

fn compiled() -> &'static Vec<(&'static str, Regex)> {
    static TABLE: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        SIGNATURES
            .iter()
            .map(|s| {
                let rx = RegexBuilder::new(s.pattern)
                    .case_insensitive(true)
                    .build()
                    .expect("static signature regex must compile");
                (s.domain, rx)
            })
            .collect()
    })
}

/// All buckets whose signature matches this line (multi-count semantics:
/// at most one entry per bucket label — miner parity with trace_miner.py).
pub fn matching_buckets(line: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for (domain, rx) in compiled().iter() {
        if rx.is_match(line) && !out.iter().any(|d| *d == *domain) {
            out.push(domain);
        }
    }
    out
}

/// First matching bucket in priority order (first-match consumers).
pub fn first_matching_bucket(line: &str) -> Option<&'static str> {
    compiled()
        .iter()
        .find(|(_, rx)| rx.is_match(line))
        .map(|(domain, _)| *domain)
}

/// Distinct bucket labels in declared order (for adapters that key maps).
pub fn bucket_domains() -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for s in SIGNATURES {
        if !seen.iter().any(|d| d == s.domain) {
            seen.push(s.domain.to_string());
        }
    }
    seen
}

/// Count of distinct rules (observability/debug).
pub fn signature_count() -> usize {
    SIGNATURES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ice_substring_trap_is_closed() {
        // These poisoned the old matcher (~87k false positives).
        assert!(matching_buckets("restart the service and check the device").is_empty());
        assert!(matching_buckets("office VPN device policy").is_empty());
        assert!(matching_buckets("voicemode service ready").is_empty());
        assert_eq!(
            first_matching_bucket("rustc SIGSEGV during codegen"),
            Some("toolchain_ice")
        );
    }

    #[test]
    fn rustc_toolchain_mention_is_not_an_event() {
        // 2026-08-25: bare `rustc`/`toolchain` mentions are NOT events —
        // same defect class as the old loose "ice" substring. A failure
        // verb or crash noun must co-occur near the subject.
        assert!(first_matching_bucket("updated rustc to 1.96.0 today").is_none());
        assert!(matching_buckets("toolchain pinned in rust-toolchain.toml").is_empty());
        assert!(first_matching_bucket("reading the rustc source for fun").is_none());
        // Real crash/failure events keep matching.
        assert_eq!(
            first_matching_bucket("error: rustc interrupted by SIGSEGV"),
            Some("toolchain_ice")
        );
        assert_eq!(
            first_matching_bucket("internal compiler error in rustc"),
            Some("toolchain_ice")
        );
        assert_eq!(
            first_matching_bucket("toolchain 1.95 failed to install"),
            Some("toolchain_ice")
        );
        assert!(matching_buckets("rustc panicked at src/lib.rs:1").contains(&"toolchain_ice"));
    }

    #[test]
    fn cargo_mention_is_not_a_block_but_failure_is() {
        // Mere mention of running cargo must NOT count (old bug: any
        // "cargo build" text was a toolchain_ice block).
        assert!(first_matching_bucket("running cargo build now").is_none());
        assert_eq!(
            first_matching_bucket("cargo build failed with SIGSEGV"),
            Some("toolchain_ice")
        );
        assert_eq!(
            first_matching_bucket("could not compile cynic-host"),
            Some("cargo_build")
        );
    }

    #[test]
    fn multi_bucket_line_counts_each_label_once() {
        let line = "rustc SIGSEGV then could not compile: permission denied on target/";
        let mut buckets = matching_buckets(line);
        buckets.sort_unstable();
        assert_eq!(buckets, vec!["cargo_build", "permission", "toolchain_ice"]);
    }

    #[test]
    fn domains_are_stable_and_covered() {
        let domains = bucket_domains();
        assert_eq!(
            domains,
            vec![
                "toolchain_ice",
                "cargo_build",
                "network",
                "oom",
                "permission",
                "notfound"
            ]
        );
        assert!(signature_count() >= domains.len());
    }

    #[test]
    fn real_corpus_samples_classify_correctly() {
        // Shapes observed verbatim in the mined corpus.
        assert_eq!(
            first_matching_bucket(
                "error: linking with `cc` failed: rust-lld: error: relocation R_X86_64_GOTPCREL"
            ),
            None
        ); // linker noise without crash words stays unclassified (honest)
        assert_eq!(
            first_matching_bucket("thread 'rustc' panicked; signal: 11, SIGSEGV"),
            Some("toolchain_ice")
        );
        assert_eq!(
            first_matching_bucket("ssh: connect to host 10.10.10.103 port 22: Connection refused"),
            Some("network")
        );
        assert_eq!(
            first_matching_bucket("fatal error: out of memory while linking"),
            Some("oom")
        );
        // Ambiguous lines legitimately match MULTIPLE buckets; first-match
        // consumers follow declared priority, counting consumers get both.
        // Event-shaped line: since the 2026-08-25 mention-vs-event fix, a
        // bare "in rustc" mention no longer counts as ICE — the crash verb
        // carries the signal.
        let ambiguous = matching_buckets("fatal error: out of memory while rustc panicked");
        assert!(ambiguous.contains(&"oom") && ambiguous.contains(&"toolchain_ice"));
    }
}
