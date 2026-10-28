//! SSH batch options and tunable SSH options.

/// SSH options every remote command carries.
pub const SSH_BATCH_OPTIONS: [&str; 6] = [
    "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes", "-o", "ConnectTimeout=10",
];

/// Tunable SSH options. Defaults match [`SSH_BATCH_OPTIONS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SshOptions {
    pub batch_mode: bool,
    pub strict_host_key_checking: bool,
    pub connect_timeout_secs: u64,
}

impl Default for SshOptions {
    fn default() -> Self {
        Self {
            batch_mode: true,
            strict_host_key_checking: true,
            connect_timeout_secs: 10,
        }
    }
}

impl SshOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn batch_mode(mut self, value: bool) -> Self {
        self.batch_mode = value;
        self
    }

    pub fn strict_host_key_checking(mut self, value: bool) -> Self {
        self.strict_host_key_checking = value;
        self
    }

    pub fn connect_timeout_secs(mut self, value: u64) -> Self {
        self.connect_timeout_secs = value;
        self
    }

    /// Render the options as argv fragments (`-o key=value`).
    pub fn to_argv(&self) -> Vec<String> {
        let mut argv = Vec::new();
        if self.batch_mode {
            argv.push("-o".to_string());
            argv.push("BatchMode=yes".to_string());
        }
        if self.strict_host_key_checking {
            argv.push("-o".to_string());
            argv.push("StrictHostKeyChecking=yes".to_string());
        }
        argv.push("-o".to_string());
        argv.push(format!("ConnectTimeout={}", self.connect_timeout_secs));
        argv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_match_batch_const() {
        let opts = SshOptions::default();
        assert!(opts.batch_mode);
        assert!(opts.strict_host_key_checking);
        assert_eq!(opts.connect_timeout_secs, 10);
        assert_eq!(opts.to_argv(), SSH_BATCH_OPTIONS.to_vec());
    }

    #[test]
    fn builder_overrides_values() {
        let opts = SshOptions::new()
            .batch_mode(false)
            .strict_host_key_checking(false)
            .connect_timeout_secs(30);
        assert!(!opts.batch_mode);
        assert!(!opts.strict_host_key_checking);
        assert_eq!(opts.connect_timeout_secs, 30);
        assert_eq!(opts.to_argv(), vec!["-o", "ConnectTimeout=30"]);
    }
}
