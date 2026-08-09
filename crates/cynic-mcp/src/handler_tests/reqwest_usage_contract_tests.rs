#[cfg(test)]
mod reqwest_usage_contract_tests {
    use super::*;

    #[test]
    fn reqwest_usage_contract_rejects_domain_reqwest_usage() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-reqwest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let domain_dir = temp.join("crates/cynic-kernel/src/domain");
        std::fs::create_dir_all(&domain_dir).expect("fixture directory should be created");
        let file = domain_dir.join("bad.rs");
        std::fs::write(&file, "use reqwest::Client;\n").expect("fixture file should be written");

        let findings = validate_reqwest_usage(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("misplaced_reqwest_usage")
        }));
    }

    #[test]
    fn reqwest_usage_contract_rejects_removed_kernel_proxy_path() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-reqwest-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let proxy_dir = temp.join("crates/cynic-kernel/src/api/mcp");
        std::fs::create_dir_all(&proxy_dir).expect("fixture directory should be created");
        let file = proxy_dir.join("proxy.rs");
        std::fs::write(&file, "use reqwest::Client;\n").expect("fixture file should be written");

        let findings = validate_reqwest_usage(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("misplaced_reqwest_usage")
        }));
    }
}

