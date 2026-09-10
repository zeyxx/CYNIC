//! Cargo-shaped Bubblewrap sandbox argv constructor.
//!
//! Unlike [`super::bwrap::build_bwrap_argv`] (git-agent workspaces rooted at
//! `/workspace` with `GIT_DIR` wiring), this constructor confines a Cargo
//! build: the toolchain and registry are read-only, only the working
//! directory and the Cargo target directory are writable, the network
//! namespace is always unshared, and the child starts in the working
//! directory. It is pure (no I/O beyond read-only path existence probes,
//! same as the git-shaped constructor) and fully unit-testable without a
//! bwrap binary on `PATH`.

use std::path::{Path, PathBuf};

/// Execution isolation mode for a bounded build.
///
/// `Off` preserves the historical bare-metal subprocess. `Bwrap` wraps the
/// admitted argv with [`build_cargo_sandbox_argv`]. The mode travels on the
/// request so the decision stays reviewable; the executor never guesses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildSandboxMode {
    Off,
    Bwrap(CargoSandboxSpec),
}

impl Default for BuildSandboxMode {
    fn default() -> Self {
        Self::Off
    }
}

/// Which directories may appear on the sandbox command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoSandboxSpec {
    /// bwrap executable. Absolute path pins the exact binary under test;
    /// `"bwrap"` resolves via `PATH` on the executor host.
    pub bwrap_bin: String,
    /// Crate workspace the build runs in. Bound read-write, used as `--chdir`.
    pub working_directory: PathBuf,
    /// Cargo target directory. Bound read-write so `cargo` can emit artifacts.
    pub target_directory: PathBuf,
    /// Cargo home (registry cache, binaries). Bound read-only.
    pub cargo_home: PathBuf,
    /// Rustup home (toolchains). Bound read-only.
    pub rustup_home: PathBuf,
    /// Extra read-only binds expressed as `(host, sandbox)` pairs.
    pub extra_ro_binds: Vec<(PathBuf, PathBuf)>,
}

/// System base directories mirrored read-only inside every cargo sandbox.
/// `/etc/alternatives` is required: toolchain binaries such as `cc` reach
/// their real executable through it, and a missing link breaks build
/// scripts with a misleading "not found".
const SYSTEM_RO_DIRS: &[&str] = &[
    "/usr",
    "/bin",
    "/lib",
    "/lib64",
    "/etc/ssl",
    "/etc/resolv.conf",
    "/etc/alternatives",
];

impl CargoSandboxSpec {
    /// Resolve a spec from the executor host environment.
    ///
    /// Reads `CYNIC_BWRAP_BIN` (default `"bwrap"`), `PATH` (required),
    /// `CARGO_HOME` (default `~/.cargo`), `RUSTUP_HOME` (default `~/.rustup`),
    /// and `CARGO_TARGET_DIR` (default `<working_directory>/target`).
    /// Returns `None` when `PATH` is missing or empty — the caller must
    /// refuse sandboxed execution instead of guessing a toolchain layout.
    pub fn resolve_from_host_env(
        working_directory: PathBuf,
        target_directory: Option<PathBuf>,
    ) -> Option<Self> {
        let path_value = std::env::var("PATH").ok()?;
        if path_value.trim().is_empty() {
            return None;
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let cargo_home = std::env::var("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".cargo"));
        let rustup_home = std::env::var("RUSTUP_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".rustup"));
        let target_directory = target_directory
            .or_else(|| std::env::var("CARGO_TARGET_DIR").map(PathBuf::from).ok())
            .unwrap_or_else(|| working_directory.join("target"));
        let path_dirs = std::env::split_paths(&path_value).collect::<Vec<_>>();
        // Only bind directories that exist: bwrap fails the whole sandbox
        // on a missing source, and PATH routinely names absent entries
        // (/snap/bin on servers, for example).
        let mut extra_ro_binds = Vec::new();
        for dir in path_dirs {
            if !dir.as_os_str().is_empty() && dir.is_dir() {
                extra_ro_binds.push((dir.clone(), dir));
            }
        }
        Some(Self {
            bwrap_bin: std::env::var("CYNIC_BWRAP_BIN")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "bwrap".to_string()),
            working_directory,
            target_directory,
            cargo_home,
            rustup_home,
            extra_ro_binds,
        })
    }

    /// Validate the spec before argv construction. Pure.
    pub fn validate(&self) -> Result<(), String> {
        if !self.working_directory.is_absolute() {
            return Err("cargo_sandbox_working_directory_not_absolute".into());
        }
        if !self.target_directory.is_absolute() {
            return Err("cargo_sandbox_target_directory_not_absolute".into());
        }
        if self.working_directory.as_os_str().is_empty() {
            return Err("cargo_sandbox_working_directory_missing".into());
        }
        Ok(())
    }
}

/// Build the exact `bwrap` argv for a Cargo invocation.
///
/// `command` is the admitted build argv (e.g. `["cargo", "check", ...]`)
/// and `passthrough_env` the bounded environment admitted for the build
/// (grant paths, stack sizes, target dir). Every entry becomes `--setenv`
/// inside the sandbox; nothing else from the host environment leaks in
/// except the reconstructed `PATH`, `HOME`, `CARGO_HOME`, `RUSTUP_HOME`,
/// and `CARGO_TARGET_DIR` below.
///
/// Layout guarantees (asserted by unit tests):
/// - `--unshare-net` is always present: the sandbox has no egress, so only
///   `--offline`/`--locked`/`--frozen` Cargo invocations can succeed.
/// - the toolchain (`PATH` dirs, `CARGO_HOME`, `RUSTUP_HOME`) is read-only.
/// - only `working_directory` and `target_directory` are writable.
/// - `"--"` separates bwrap options from the build command so leading-dash
///   Cargo flags (`-p`, `--frozen`) can never be parsed as bwrap options.
pub fn build_cargo_sandbox_argv(
    spec: &CargoSandboxSpec,
    command: &[String],
    passthrough_env: &[(String, String)],
) -> Result<Vec<String>, String> {
    spec.validate()?;
    let (program, _arguments) = command
        .split_first()
        .ok_or_else(|| "cargo_sandbox_command_missing".to_string())?;
    if program.trim().is_empty() {
        return Err("cargo_sandbox_command_missing".to_string());
    }
    let mut argv = vec![
        spec.bwrap_bin.clone(),
        "--die-with-parent".to_string(),
        "--unshare-pid".to_string(),
        "--unshare-uts".to_string(),
        "--unshare-ipc".to_string(),
        "--unshare-net".to_string(),
    ];
    for dir in SYSTEM_RO_DIRS {
        if Path::new(dir).exists() {
            argv.push("--ro-bind".to_string());
            argv.push(dir.to_string());
            argv.push(dir.to_string());
        }
    }
    for (host, sandbox) in &spec.extra_ro_binds {
        if host.as_os_str().is_empty() || sandbox.as_os_str().is_empty() {
            return Err("cargo_sandbox_ro_bind_missing_path".to_string());
        }
        argv.push("--ro-bind".to_string());
        argv.push(host.display().to_string());
        argv.push(sandbox.display().to_string());
    }
    // Toolchain homes are required for any cargo run: always emitted so a
    // missing home fails loudly at spawn with the exact path, never silently
    // dropped into a broken toolchain view.
    for home in [&spec.cargo_home, &spec.rustup_home] {
        argv.push("--ro-bind".to_string());
        argv.push(home.display().to_string());
        argv.push(home.display().to_string());
    }
    for writable in [&spec.working_directory, &spec.target_directory] {
        argv.push("--bind".to_string());
        argv.push(writable.display().to_string());
        argv.push(writable.display().to_string());
    }
    argv.push("--tmpfs".to_string());
    argv.push("/tmp".to_string());
    argv.push("--proc".to_string());
    argv.push("/proc".to_string());
    argv.push("--dev".to_string());
    argv.push("/dev".to_string());
    argv.push("--chdir".to_string());
    argv.push(spec.working_directory.display().to_string());
    argv.push("--setenv".to_string());
    argv.push("HOME".to_string());
    argv.push("/tmp".to_string());
    let path_value = spec
        .extra_ro_binds
        .iter()
        .map(|(host, _)| host.display().to_string())
        .collect::<Vec<_>>()
        .join(":");
    argv.push("--setenv".to_string());
    argv.push("PATH".to_string());
    argv.push(path_value);
    argv.push("--setenv".to_string());
    argv.push("CARGO_HOME".to_string());
    argv.push(spec.cargo_home.display().to_string());
    argv.push("--setenv".to_string());
    argv.push("RUSTUP_HOME".to_string());
    argv.push(spec.rustup_home.display().to_string());
    argv.push("--setenv".to_string());
    argv.push("CARGO_TARGET_DIR".to_string());
    argv.push(spec.target_directory.display().to_string());
    for (key, value) in passthrough_env {
        if key.trim().is_empty() || key.contains('=') || key.contains('\0') {
            return Err("cargo_sandbox_env_key_invalid".to_string());
        }
        argv.push("--setenv".to_string());
        argv.push(key.clone());
        argv.push(value.clone());
    }
    argv.push("--".to_string());
    argv.extend(command.iter().cloned());
    Ok(argv)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_spec() -> CargoSandboxSpec {
        CargoSandboxSpec {
            bwrap_bin: "bwrap".to_string(),
            working_directory: PathBuf::from("/work/checkout"),
            target_directory: PathBuf::from("/work/target-sandbox"),
            cargo_home: PathBuf::from("/home/builder/.cargo"),
            rustup_home: PathBuf::from("/home/builder/.rustup"),
            extra_ro_binds: vec![
                (
                    PathBuf::from("/home/builder/.cargo/bin"),
                    PathBuf::from("/home/builder/.cargo/bin"),
                ),
                (
                    PathBuf::from("/usr/local/bin"),
                    PathBuf::from("/usr/local/bin"),
                ),
            ],
        }
    }

    fn fixture_command() -> Vec<String> {
        vec![
            "cargo".to_string(),
            "check".to_string(),
            "-p".to_string(),
            "cynic-protocol".to_string(),
            "--frozen".to_string(),
        ]
    }

    #[test]
    fn sandbox_argv_isolates_network_and_confines_writes() {
        let argv = build_cargo_sandbox_argv(&fixture_spec(), &fixture_command(), &[]).unwrap();
        assert_eq!(argv[0], "bwrap");
        assert!(argv.contains(&"--unshare-net".to_string()));
        assert!(argv.contains(&"--unshare-pid".to_string()));
        // No share-net anywhere.
        assert!(!argv.contains(&"--share-net".to_string()));
        // Writable binds cover exactly working + target dir.
        let binds = argv
            .windows(3)
            .filter(|w| w[0] == "--bind")
            .map(|w| w[1].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            binds,
            vec![
                "/work/checkout".to_string(),
                "/work/target-sandbox".to_string()
            ]
        );
        // Homes are read-only.
        assert!(argv.windows(3).any(|w| w[0] == "--ro-bind"
            && w[1] == "/home/builder/.cargo"
            && w[2] == "/home/builder/.cargo"));
        // Sandbox starts in the working directory.
        let chdir_pos = argv.iter().position(|a| a == "--chdir").unwrap();
        assert_eq!(argv[chdir_pos + 1], "/work/checkout");
        // Command follows an explicit option terminator.
        let sep = argv.iter().position(|a| a == "--").unwrap();
        assert_eq!(&argv[sep + 1..], fixture_command().as_slice());
    }

    #[test]
    fn sandbox_argv_rejects_relative_working_directory() {
        let mut spec = fixture_spec();
        spec.working_directory = PathBuf::from("relative/path");
        assert!(build_cargo_sandbox_argv(&spec, &fixture_command(), &[]).is_err());
    }

    #[test]
    fn sandbox_argv_rejects_empty_command() {
        assert!(build_cargo_sandbox_argv(&fixture_spec(), &[], &[]).is_err());
    }

    #[test]
    fn sandbox_argv_rejects_malformed_env_keys() {
        let bad = vec![("HAS=EQUALS".to_string(), "x".to_string())];
        assert!(build_cargo_sandbox_argv(&fixture_spec(), &fixture_command(), &bad).is_err());
    }

    #[test]
    fn sandbox_argv_passes_bounded_environment_inside() {
        let env = vec![
            ("RUST_MIN_STACK".to_string(), "8589934592".to_string()),
            ("CARGO_INCREMENTAL".to_string(), "0".to_string()),
        ];
        let argv = build_cargo_sandbox_argv(&fixture_spec(), &fixture_command(), &env).unwrap();
        for (key, value) in &env {
            let pos = argv
                .windows(3)
                .position(|w| w[0] == "--setenv" && w[1] == *key && w[2] == *value)
                .unwrap();
            let _ = pos;
        }
        assert!(argv.contains(&"HOME".to_string()));
    }
}
