#[cfg(test)]
mod secrets_contract_tests {
    use super::*;

    #[test]
    fn secrets_contract_rejects_raw_secret() {
        let findings = secret_findings_from_diff("+API_KEY=sk-abcdefghijklmnopqrstuvwxyz\n");

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("raw_secret_exposed")
        }));
    }

    #[test]
    fn secrets_contract_rejects_hardcoded_credential() {
        let findings = secret_findings_from_diff("+token: supersecretvalue\n");

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("hardcoded_credential_exposed")
        }));
    }

    #[test]
    fn secrets_contract_allows_excluded_patterns() {
        let findings = secret_findings_from_diff("+api_key=api_key\n+token?: string\n");

        assert!(findings.is_empty());
    }
}

