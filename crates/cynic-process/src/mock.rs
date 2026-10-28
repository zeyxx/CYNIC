use std::collections::HashMap;

use cynic_protocol::argv::{ArgvOutput, ArgvRunner};

/// Test double: returns fixed JSON for known commands.
#[derive(Debug, Default, Clone)]
pub struct MapArgvRunner {
    pub responses: Vec<(Vec<String>, ArgvOutput)>,
}

impl MapArgvRunner {
    /// Build a test runner from shell-like command keys.
    pub fn new(responses: HashMap<String, ArgvOutput>) -> Self {
        Self {
            responses: responses
                .into_iter()
                .map(|(command, output)| {
                    (
                        command.split_whitespace().map(str::to_string).collect(),
                        output,
                    )
                })
                .collect(),
        }
    }
}

impl ArgvRunner for MapArgvRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        let key: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
        for (k, v) in &self.responses {
            if k == &key {
                return v.clone();
            }
        }
        // prefix match: first N tokens
        for (k, v) in &self.responses {
            if key.len() >= k.len() && key[..k.len()] == k[..] {
                return v.clone();
            }
        }
        ArgvOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: format!("MapArgvRunner: no stub for {argv:?}"),
            spawn_error: Some(format!("MapArgvRunner: no stub for {argv:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::argv::ArgvRunner;
    use std::collections::HashMap;

    #[test]
    fn stub_exact() {
        let mut m = HashMap::new();
        m.insert(
            "echo hi".to_string(),
            ArgvOutput {
                exit_code: Some(0),
                stdout: "hi".into(),
                stderr: String::new(),
                spawn_error: None,
            },
        );
        let r = MapArgvRunner::new(m);
        let out = r.run(&["echo", "hi"]);
        assert!(out.success());
        assert_eq!(out.stdout, "hi");
    }

    #[test]
    fn fallback_no_stub() {
        let r = MapArgvRunner::default();
        let out = r.run(&["nope"]);
        assert!(!out.success());
        assert!(out.spawn_error.is_some());
    }
}
