use std::collections::HashMap;
use std::path::PathBuf;

use super::embedded_domains;

/// Load domain-specific axiom evaluation prompts.
///
/// Priority:
/// - `CYNIC_DOMAINS_DIR` (override)
/// - `/etc/cynic/domains` (runtime-authority)
/// - embedded prompts (compile-time fallback)
pub fn load_domain_prompts() -> HashMap<String, String> {
    let mut prompts = HashMap::new();

    // Runtime-authority first, then explicit env override.
    let domains_dir = std::env::var("CYNIC_DOMAINS_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .or_else(|| {
            let canonical = std::path::Path::new("/etc/cynic/domains");
            if canonical.is_dir() {
                Some(canonical.to_path_buf())
            } else {
                None
            }
        });

    if let Some(domains_dir) = domains_dir {
        if domains_dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(&domains_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let stripped = embedded_domains::strip_domain_heading(&content);
                        if !stripped.is_empty() {
                            crate::klog!(
                                "[config] Domain prompt loaded (filesystem): '{}' ({} chars)",
                                name,
                                stripped.len()
                            );
                            prompts.insert(name.to_string(), stripped);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "failed to read domain prompt file"
                        );
                    }
                }
            }
        }
    } else {
        tracing::warn!("no filesystem domain prompt directory configured, using embedded fallback");
    }

    // Fallback: embedded prompts for any domain not found on filesystem.
    let embedded = embedded_domains::load_embedded_domain_prompts();
    let mut fallback_count = 0;
    for (name, content) in embedded {
        prompts.entry(name).or_insert_with(|| {
            fallback_count += 1;
            content
        });
    }

    if prompts.is_empty() {
        crate::klog!("[config] No domain prompts found — using generic prompts");
    } else {
        let fs_count = prompts.len() - fallback_count;
        crate::klog!(
            "[config] {} domain prompt(s) loaded ({} filesystem, {} embedded fallback)",
            prompts.len(),
            fs_count,
            fallback_count
        );
    }
    prompts
}
