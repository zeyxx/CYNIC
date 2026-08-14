use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelEndpointPolicy {
    pub allow_empty: bool,
    pub reject_loopback: bool,
    pub allow_loopback: bool,
}

impl Default for KernelEndpointPolicy {
    fn default() -> Self {
        Self {
            allow_empty: false,
            reject_loopback: true,
            allow_loopback: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelEndpointError {
    MissingBaseUrl,
    LoopbackRejected,
}

impl fmt::Display for KernelEndpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBaseUrl => {
                write!(f, "CYNIC_REST_ADDR is required (SSOT kernel endpoint).")
            }
            Self::LoopbackRejected => {
                write!(f, "Loopback kernel endpoint rejected by SSOT policy.")
            }
        }
    }
}

impl std::error::Error for KernelEndpointError {}

pub fn normalize_kernel_base_url(
    raw: &str,
    policy: KernelEndpointPolicy,
) -> Result<String, KernelEndpointError> {
    let value = raw.trim();
    if value.is_empty() {
        return if policy.allow_empty {
            Ok(String::new())
        } else {
            Err(KernelEndpointError::MissingBaseUrl)
        };
    }

    let mut normalized = value.to_string();
    if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
        normalized = format!("http://{normalized}");
    }

    let host = host_from_url(&normalized);
    let is_loopback = host == "localhost" || host.starts_with("127.");
    if policy.reject_loopback && is_loopback && !policy.allow_loopback {
        return if policy.allow_empty {
            Ok(String::new())
        } else {
            Err(KernelEndpointError::LoopbackRejected)
        };
    }

    while normalized.ends_with('/') {
        normalized.pop();
    }
    Ok(normalized)
}

pub fn kernel_url(
    base_url: &str,
    path: &str,
    policy: KernelEndpointPolicy,
) -> Result<String, KernelEndpointError> {
    let base = normalize_kernel_base_url(base_url, policy)?;
    if base.is_empty() {
        return Ok(String::new());
    }
    if path.starts_with('/') {
        Ok(format!("{base}{path}"))
    } else {
        Ok(format!("{base}/{path}"))
    }
}

pub fn auth_header_value(api_key: &str) -> Option<String> {
    let token = api_key.trim();
    if token.is_empty() {
        None
    } else {
        Some(format!("Bearer {token}"))
    }
}

fn host_from_url(url: &str) -> &str {
    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    let host_port = without_scheme.split('/').next().unwrap_or_default();
    host_port
        .strip_prefix('[')
        .and_then(|value| value.split(']').next())
        .unwrap_or_else(|| host_port.split(':').next().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_missing_scheme_and_trailing_slash() {
        let url = normalize_kernel_base_url("kernel.local:3030/", KernelEndpointPolicy::default())
            .expect("url should normalize");
        assert_eq!(url, "http://kernel.local:3030");
    }

    #[test]
    fn rejects_loopback_by_default() {
        let err = normalize_kernel_base_url("127.0.0.1:3030", KernelEndpointPolicy::default())
            .expect_err("loopback should be rejected");
        assert_eq!(err, KernelEndpointError::LoopbackRejected);
    }

    #[test]
    fn allows_loopback_when_policy_explicitly_enables_it() {
        let url = normalize_kernel_base_url(
            "http://127.0.0.1:3030",
            KernelEndpointPolicy {
                allow_loopback: true,
                ..KernelEndpointPolicy::default()
            },
        )
        .expect("loopback should be allowed");
        assert_eq!(url, "http://127.0.0.1:3030");
    }

    #[test]
    fn builds_kernel_url_with_missing_leading_slash() {
        let url = kernel_url(
            "kernel.local:3030",
            "observe",
            KernelEndpointPolicy::default(),
        )
        .expect("kernel url should build");
        assert_eq!(url, "http://kernel.local:3030/observe");
    }

    #[test]
    fn formats_auth_header_when_token_present() {
        assert_eq!(
            auth_header_value("secret").as_deref(),
            Some("Bearer secret")
        );
        assert_eq!(auth_header_value("   "), None);
    }
}
