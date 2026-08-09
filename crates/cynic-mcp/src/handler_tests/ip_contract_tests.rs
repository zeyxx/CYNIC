#[cfg(test)]
mod ip_contract_tests {
    use super::*;

    #[test]
    fn ip_contract_rejects_staged_hardcoded_ip() {
        let findings = ip_findings_from_diff("+server = \"100.74.12.5\"\n");

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("staged_hardcoded_ip")
        }));
    }

    #[test]
    fn ip_contract_rejects_backends_hardcoded_ip() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-ips-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(temp.join("backends.toml"), "host = \"100.75.1.8\"\n")
            .expect("fixture file should be written");

        let findings = ip_findings_from_backends(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("backends_hardcoded_ip")
        }));
    }

    #[test]
    fn ip_contract_accepts_no_ip() {
        let findings = ip_findings_from_diff("+server = env(\"HOST\")\n");
        assert!(findings.is_empty());
    }
}

