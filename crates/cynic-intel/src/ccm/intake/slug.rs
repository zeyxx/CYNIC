//! Semantic slug generation across operational and inference domains.

/// Reduce content to a domain-aware "semantic slug" before FNV hashing.
pub fn semantic_slug(domain: &str, content: &str) -> String {
    let slug = match domain {
        "dev" => slug_dev(content),
        "token" | "token-analysis" => slug_token(content),
        "session" => slug_session(content),
        "trading" => slug_trading(content),
        "D2" => slug_inference(content),
        "chess" => return format!("chess:{content}"),
        "governance" => slug_governance(content),
        _ => slug_general(content),
    };
    format!("{domain}:{slug}")
}

/// Dev commits: extract conventional commit type + scope.
fn slug_dev(content: &str) -> String {
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

    let types = [
        "feat", "fix", "refactor", "chore", "docs", "test", "style", "perf", "ci", "build",
    ];

    let lower = stripped.to_lowercase();
    for t in types {
        if lower.starts_with(t) {
            let rest = &stripped[t.len()..];
            if let Some(rest_after_paren) = rest.strip_prefix('(')
                && let Some(end) = rest_after_paren.find(')')
            {
                let scope = &rest_after_paren[..end];
                return format!("{t}:{scope}").to_lowercase();
            }
            if let Some(file) = extract_file_stem(rest) {
                return format!("{t}:{file}").to_lowercase();
            }
            return t.to_string();
        }
    }

    first_n_words(stripped, 3)
}

/// Token analysis: extract structural risk profile from enriched stimulus.
fn slug_token(content: &str) -> String {
    let mint_auth = extract_field_status(content, "mint_authority");
    let freeze = extract_field_status(content, "freeze_authority");
    let meta = if content.contains("metadata_mutable: YES")
        || content.contains("metadata_mutable: true")
    {
        "mutable"
    } else if content.contains("metadata_mutable: NO")
        || content.contains("metadata_mutable: false")
    {
        "immutable"
    } else {
        "unk"
    };

    let profile = format!("{mint_auth}:{freeze}:{meta}");

    if profile == "unk:unk:unk" {
        return first_n_words(content, 3);
    }

    profile
}

/// Extract REVOKED/ACTIVE/unk from a field.
fn extract_field_status(content: &str, field: &str) -> &'static str {
    if let Some(pos) = content.find(field) {
        let after = &content[pos + field.len()..];
        let window = &after[..after.len().min(40)];
        if window.contains("REVOKED") {
            "revoked"
        } else if window.contains("ACTIVE") {
            "active"
        } else {
            "unk"
        }
    } else {
        "unk"
    }
}

/// Session: collapse to agent type only.
fn slug_session(content: &str) -> String {
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
fn slug_trading(content: &str) -> String {
    let lower = content.to_lowercase();

    let side = if lower.contains("long") {
        "long"
    } else if lower.contains("short") {
        "short"
    } else if lower.contains("buy") {
        "buy"
    } else if lower.contains("sell") {
        "sell"
    } else {
        lower.split_whitespace().next().unwrap_or("unknown")
    };

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

/// D2 inference domain: extract model family or engine from tweet content.
fn slug_inference(content: &str) -> String {
    let lower = content.to_lowercase();

    let model = [
        "qwen",
        "llama",
        "gemma",
        "mistral",
        "phi",
        "claude",
        "gpt",
        "deepseek",
        "command-r",
    ]
    .iter()
    .find(|m| lower.contains(**m))
    .copied();

    let engine = [
        ("vllm", "vllm"),
        ("sglang", "sglang"),
        ("llama.cpp", "llamacpp"),
        ("llama-server", "llamacpp"),
        ("exllama", "exllama"),
        ("tensorrt", "tensorrt"),
        ("mlx", "mlx"),
        ("ollama", "ollama"),
        ("gguf", "llamacpp"),
    ]
    .iter()
    .find(|(pat, _)| lower.contains(pat))
    .map(|(_, slug)| *slug);

    let topic = [
        ("benchmark", "bench"),
        ("tok/s", "bench"),
        ("tokens per second", "bench"),
        ("quantiz", "quant"),
        ("iq3", "quant"),
        ("iq4", "quant"),
        ("q4_k", "quant"),
        ("q6_k", "quant"),
        ("speculative", "spec"),
        ("mtp", "spec"),
        ("draft", "spec"),
        ("serving", "serve"),
        ("deploy", "serve"),
        ("throughput", "serve"),
        ("vram", "memory"),
        ("bandwidth", "memory"),
        ("gpu memory", "memory"),
        ("fine-tun", "train"),
        ("lora", "train"),
        ("training", "train"),
    ]
    .iter()
    .find(|(pat, _)| lower.contains(pat))
    .map(|(_, slug)| *slug);

    match (model, engine, topic) {
        (Some(m), Some(e), _) => format!("{m}:{e}"),
        (Some(m), None, Some(t)) => format!("{m}:{t}"),
        (Some(m), None, None) => m.to_string(),
        (None, Some(e), Some(t)) => format!("{e}:{t}"),
        (None, Some(e), None) => e.to_string(),
        (None, None, Some(t)) => t.to_string(),
        (None, None, None) => first_n_words(content, 3),
    }
}

/// Governance proposals: slug by action-type + subject.
fn slug_governance(content: &str) -> String {
    let lower = content.to_lowercase();
    let action = if lower.contains("hire") || lower.contains("appoint") || lower.contains("elect") {
        "hire"
    } else if lower.contains("fire") || lower.contains("remove") || lower.contains("terminate") {
        "remove"
    } else if lower.contains("fund")
        || lower.contains("grant")
        || lower.contains("allocat")
        || lower.contains("budget")
        || lower.contains("spend")
    {
        "spend"
    } else if lower.contains("upgrade") || lower.contains("deploy") || lower.contains("migrate") {
        "upgrade"
    } else if lower.contains("change")
        || lower.contains("update")
        || lower.contains("modify")
        || lower.contains("parameter")
    {
        "param"
    } else if lower.contains("partner")
        || lower.contains("integrat")
        || lower.contains("collaborat")
    {
        "partner"
    } else {
        "propose"
    };
    let subject = if lower.contains("adviser")
        || lower.contains("advisor")
        || lower.contains("consultant")
    {
        "adviser"
    } else if lower.contains("treasury") || lower.contains("usdc") {
        "treasury"
    } else if lower.contains("developer") || lower.contains("engineer") || lower.contains("team") {
        "team"
    } else if lower.contains("protocol") || lower.contains("contract") || lower.contains("program")
    {
        "protocol"
    } else if lower.contains("parameter") || lower.contains("threshold") || lower.contains("quorum")
    {
        "param"
    } else {
        "general"
    };
    format!("{action}:{subject}")
}

fn slug_general(content: &str) -> String {
    if let Some(after_bracket) = content.strip_prefix('[').and_then(|s| s.split_once("] ")) {
        let rest = after_bracket.1;
        let tool = rest.split_whitespace().next().unwrap_or("");
        let tool_lower = tool.to_lowercase();

        let known_tools = [
            "bash",
            "read",
            "write",
            "edit",
            "grep",
            "glob",
            "boot",
            "dog_failed",
            "dog_expired",
            "dog_discovered",
            "storage_reconnected",
            "anomaly",
            "session_distill",
            "session_compliance",
            "session_temporal",
            "user_prompt",
        ];

        for known in known_tools {
            if tool_lower == known || tool_lower.starts_with(known) {
                return format!("tool:{known}");
            }
        }

        return format!("tool:{tool_lower}");
    }

    first_n_words(content, 3)
}

/// Generic: first N significant words, lowercased.
fn first_n_words(content: &str, n: usize) -> String {
    content
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .take(n)
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase()
}

/// Extract a Rust/source file stem from text.
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
