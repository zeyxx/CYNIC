#[cfg(test)]
mod http_query_tests {
    use super::*;

    #[test]
    fn query_url_percent_encodes_query_values() {
        let url = query_url(
            "http://kernel.local",
            "/agent-tasks",
            &[
                ("kind", "hermes".to_string()),
                ("domain", "gate 2/request&deploy".to_string()),
            ],
        )
        .expect("query url should parse");

        assert_eq!(
            url,
            "http://kernel.local/agent-tasks?kind=hermes&domain=gate+2%2Frequest%26deploy"
        );
    }
}

