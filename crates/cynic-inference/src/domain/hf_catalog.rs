//! HF discovery catalog: categories, short aliases, hardware ranking.
//!
//! Operators rarely want to type full `org/Weird-Model-Name-GGUF` paths.
//! Categories map to Hub search filters; results get short aliases + fit scores.

use serde::{Deserialize, Serialize};

use super::hardware_profile::HardwareProfile;

/// Operator-facing model categories (not HF pipeline tags 1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCategory {
    Chat,
    Code,
    Embedding,
    Reasoning,
    Vision,
    Small,
    Multilingual,
    Uncensored,
    /// Ultra-low bit / ternary GGUF of large models (phone-class quants, e.g. 27B@1–2bit).
    Ultra,
    /// Any GGUF-friendly text model.
    Gguf,
}

impl ModelCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Code => "code",
            Self::Embedding => "embedding",
            Self::Reasoning => "reasoning",
            Self::Vision => "vision",
            Self::Small => "small",
            Self::Multilingual => "multilingual",
            Self::Uncensored => "uncensored",
            Self::Ultra => "ultra",
            Self::Gguf => "gguf",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "chat" | "instruct" | "assistant" => Some(Self::Chat),
            "code" | "coder" | "coding" => Some(Self::Code),
            "embed" | "embedding" | "embeddings" => Some(Self::Embedding),
            "reason" | "reasoning" | "think" | "thinking" => Some(Self::Reasoning),
            "vision" | "vlm" | "multimodal" => Some(Self::Vision),
            "small" | "tiny" => Some(Self::Small),
            "multilingual" | "i18n" | "multi" => Some(Self::Multilingual),
            "uncensored" | "abliterated" => Some(Self::Uncensored),
            "ultra" | "phone" | "mobile" | "1bit" | "1-bit" | "2bit" | "ternary" | "iq1"
            | "iq2" => Some(Self::Ultra),
            "gguf" | "llama.cpp" | "llamacpp" => Some(Self::Gguf),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Chat,
            Self::Code,
            Self::Embedding,
            Self::Reasoning,
            Self::Vision,
            Self::Small,
            Self::Multilingual,
            Self::Uncensored,
            Self::Ultra,
            Self::Gguf,
        ]
    }

    /// Hub `--search` string for this category.
    pub fn search_query(self, extra: Option<&str>) -> String {
        let base = match self {
            Self::Chat => "instruct",
            Self::Code => "coder",
            Self::Embedding => "embedding",
            Self::Reasoning => "reasoning",
            Self::Vision => "vision",
            Self::Small => "instruct",
            Self::Multilingual => "multilingual",
            Self::Uncensored => "uncensored",
            // ternary / 1-bit / 2-bit large models (phone-class quants)
            // Keep query simple — Hub search is not full boolean IR.
            Self::Ultra => "ternary bonsai 27B",
            Self::Gguf => "",
        };
        match extra {
            Some(e) if !e.trim().is_empty() && !base.is_empty() => format!("{base} {e}"),
            Some(e) if !e.trim().is_empty() => e.to_string(),
            _ if base.is_empty() => String::new(),
            _ => base.to_string(),
        }
    }

    /// Hub `--filter` tags (may be empty).
    pub fn hub_filters(self) -> Vec<&'static str> {
        match self {
            Self::Chat => vec!["gguf", "text-generation"],
            Self::Code => vec!["gguf"],
            Self::Embedding => vec!["gguf"],
            Self::Reasoning => vec!["gguf"],
            Self::Vision => vec!["gguf"],
            Self::Small => vec!["gguf"],
            Self::Multilingual => vec!["gguf"],
            Self::Uncensored => vec!["gguf"],
            Self::Ultra => vec!["gguf"],
            Self::Gguf => vec!["gguf"],
        }
    }

    /// Suggested parameter range for hardware tier (Hub --num-parameters).
    pub fn num_parameters_for_tier(self, tier: &str) -> Option<&'static str> {
        // Embeddings stay small regardless
        if matches!(self, Self::Embedding) {
            return Some("min:0B,max:2B");
        }
        if matches!(self, Self::Small) {
            return Some("min:0B,max:4B");
        }
        // Ultra-quant: large param counts; omit tight Hub param filter (many
        // ternary repos lack structured param metadata and get dropped).
        if matches!(self, Self::Ultra) {
            return None;
        }
        match tier {
            "tiny" => Some("min:0B,max:3B"),
            "small" => Some("min:0B,max:8B"),
            "medium" => Some("min:1B,max:14B"),
            "large" => Some("min:3B,max:32B"),
            "xlarge" => Some("min:7B,max:72B"),
            _ => Some("min:0B,max:14B"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub id: String,
    pub description: String,
    pub example_search: String,
}

pub fn category_catalog() -> Vec<CategoryInfo> {
    ModelCategory::all()
        .iter()
        .map(|c| CategoryInfo {
            id: c.as_str().to_string(),
            description: match c {
                ModelCategory::Chat => "General chat / instruct GGUF".into(),
                ModelCategory::Code => "Code / coder models".into(),
                ModelCategory::Embedding => "Embedding models (small)".into(),
                ModelCategory::Reasoning => "Reasoning / thinking models".into(),
                ModelCategory::Vision => "Vision / multimodal".into(),
                ModelCategory::Small => "Tiny models for tight RAM".into(),
                ModelCategory::Multilingual => "Multilingual chat".into(),
                ModelCategory::Uncensored => "Uncensored fine-tunes (operator policy)".into(),
                ModelCategory::Ultra => {
                    "Large models (14–40B) ultra-quantized 1–2bit/ternary — phone-class weights"
                        .into()
                }
                ModelCategory::Gguf => "Any GGUF on the Hub".into(),
            },
            example_search: c.search_query(None),
        })
        .collect()
}

/// Short operator alias from a Hub repo id (human-typable).
pub fn short_alias_from_repo(repo_id: &str) -> String {
    let name = repo_id.rsplit('/').next().unwrap_or(repo_id);
    let mut s = name.to_ascii_lowercase();
    for token in [
        "-gguf",
        "_gguf",
        ".gguf",
        "-ggml",
        "-instruct",
        "-chat",
        "-hf",
    ] {
        s = s.replace(token, "");
    }
    s = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}

/// Fit label relative to hardware budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareFit {
    Comfortable,
    Tight,
    OverBudget,
    Unknown,
}

pub fn score_fit(hw: &HardwareProfile, approx_size_bytes: Option<u64>) -> (HardwareFit, u32) {
    let Some(size) = approx_size_bytes else {
        return (HardwareFit::Unknown, 50);
    };
    if hw.fits_recommended(size) {
        // smaller within budget ranks slightly higher
        let ratio = size as f64 / hw.recommended_max_weight_bytes.max(1) as f64;
        let score = 100 - (ratio * 20.0) as u32;
        (HardwareFit::Comfortable, score.clamp(70, 100))
    } else if hw.fits_absolute(size) {
        (HardwareFit::Tight, 55)
    } else {
        (HardwareFit::OverBudget, 15)
    }
}

/// Rough size from parameter string like "7B" and quant tag in tags.
pub fn estimate_gguf_bytes(num_params_hint: Option<&str>, tags: &[String]) -> Option<u64> {
    let params = num_params_hint
        .and_then(parse_params_b)
        .or_else(|| tags.iter().find_map(|t| parse_params_from_tag(t)))?;
    let bpw = quant_bits_per_weight(tags).unwrap_or(4.5);
    // params * bits/weight / 8 + 10% overhead
    let bytes = (params * bpw / 8.0) * 1.10;
    Some(bytes as u64)
}

fn parse_params_b(s: &str) -> Option<f64> {
    let s = s.trim().to_ascii_uppercase();
    let s = s.trim_end_matches('B');
    s.parse::<f64>().ok().map(|n| n * 1e9)
}

fn parse_params_from_tag(tag: &str) -> Option<f64> {
    // e.g. "7B", "27b", "1.5B" — also scan inside longer tags like "qwen3.6-27b"
    let t = tag.trim();
    // Prefer whole-tag param labels
    let upper = t.to_ascii_uppercase();
    if upper.ends_with('B') && upper.len() < 8 {
        let num = &upper[..upper.len() - 1];
        if let Ok(n) = num.parse::<f64>() {
            if n > 0.0 && n < 1000.0 {
                return Some(n * 1e9);
            }
        }
    }
    // Fallback: find NNB patterns in the string
    let mut best = None;
    let bytes = upper.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'B' {
                if let Ok(n) = upper[start..i].parse::<f64>() {
                    if (1.0..200.0).contains(&n) {
                        best = Some(n * 1e9);
                    }
                }
            }
        }
        i += 1;
    }
    best
}

fn quant_bits_per_weight(tags: &[String]) -> Option<f64> {
    let joined = tags.join(" ").to_ascii_lowercase();
    // Phone-class / extreme quants first (order matters)
    if joined.contains("ternary") || joined.contains("1.58") || joined.contains("1-bit") {
        Some(1.6)
    } else if joined.contains("iq1") || joined.contains("q1_0") || joined.contains("q1_") {
        Some(2.0)
    } else if joined.contains("iq2") || joined.contains("2-bit") || joined.contains("tq2") {
        Some(2.3)
    } else if joined.contains("q2") {
        Some(2.5)
    } else if joined.contains("q3") || joined.contains("3-bit") {
        Some(3.5)
    } else if joined.contains("q4") || joined.contains("4-bit") {
        Some(4.5)
    } else if joined.contains("q5") {
        Some(5.5)
    } else if joined.contains("q6") {
        Some(6.5)
    } else if joined.contains("q8") {
        Some(8.5)
    } else if joined.contains("f16") || joined.contains("fp16") {
        Some(16.0)
    } else if joined.contains("bf16") {
        Some(16.0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_alias_strips_gguf_noise() {
        assert_eq!(
            short_alias_from_repo("prism-ml/Bonsai-27B-gguf"),
            "bonsai-27b"
        );
        assert_eq!(
            short_alias_from_repo("Qwen/Qwen2.5-7B-Instruct-GGUF"),
            "qwen2-5-7b"
        );
    }

    #[test]
    fn category_parse_aliases() {
        assert_eq!(ModelCategory::parse("coder"), Some(ModelCategory::Code));
        assert_eq!(ModelCategory::parse("gguf"), Some(ModelCategory::Gguf));
    }
}
