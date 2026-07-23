//! Static dependency tracking configuration for VersionProbe.

#[derive(Debug, Clone, Copy)]
pub(super) struct DependencySpec {
    pub name: &'static str,
    pub repo: &'static str,
    pub version_cmd: Option<&'static [&'static str]>,
}

/// Tracked dependencies: (name, github_repo, version_command).
pub(super) const TRACKED_DEPENDENCIES: &[DependencySpec] = &[
    DependencySpec {
        name: "llama.cpp",
        repo: "ggml-org/llama.cpp",
        version_cmd: None, // checked via external path; no local --version probe.
    },
    DependencySpec {
        name: "tailscale",
        repo: "tailscale/tailscale",
        version_cmd: Some(&["tailscale", "version"]),
    },
    DependencySpec {
        name: "surrealdb",
        repo: "surrealdb/surrealdb",
        version_cmd: Some(&["surreal", "version"]),
    },
];

pub(super) const GITHUB_API: &str = "https://api.github.com/repos";
