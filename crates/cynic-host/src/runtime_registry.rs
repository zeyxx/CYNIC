use std::env;
use std::path::{Path, PathBuf};

use crate::repo_access::current_project_root;
use crate::runtime_authority::RuntimeProjectionClass;

const ALLOW_LOCAL_DEV_BUILD_ENV: &str = "CYNIC_ALLOW_LOCAL_DEV_BUILD";

pub const DEFAULT_REGISTRY_PATH: &str = "/etc/cynic/registry.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRegistryMode {
    ExplicitPath,
    Environment,
    InstalledAuthority,
    DevWorktreeFallback,
}

impl RuntimeRegistryMode {
    pub fn projection_class(self) -> RuntimeProjectionClass {
        match self {
            Self::InstalledAuthority => RuntimeProjectionClass::RuntimeAuthority,
            Self::ExplicitPath | Self::Environment => RuntimeProjectionClass::OperatorProjection,
            Self::DevWorktreeFallback => RuntimeProjectionClass::DevWorktreeFallback,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitPath => "explicit_path",
            Self::Environment => "environment",
            Self::InstalledAuthority => "installed_authority",
            Self::DevWorktreeFallback => "dev_worktree_fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntimeRegistry {
    pub path: PathBuf,
    pub mode: RuntimeRegistryMode,
    pub projection_class: RuntimeProjectionClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProjectRootMode {
    DevWorktreeFallback,
    CurrentDirectoryFallback,
}

impl RuntimeProjectRootMode {
    pub fn projection_class(self) -> RuntimeProjectionClass {
        match self {
            Self::DevWorktreeFallback | Self::CurrentDirectoryFallback => {
                RuntimeProjectionClass::DevWorktreeFallback
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DevWorktreeFallback => "dev_worktree_fallback",
            Self::CurrentDirectoryFallback => "current_directory_fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntimeProjectRoot {
    pub path: PathBuf,
    pub mode: RuntimeProjectRootMode,
    pub projection_class: RuntimeProjectionClass,
}

pub fn resolve_runtime_registry_path(explicit_path: Option<&Path>) -> Result<PathBuf, String> {
    Ok(resolve_runtime_registry(explicit_path)?.path)
}

pub fn resolve_runtime_project_root() -> Option<ResolvedRuntimeProjectRoot> {
    if !allow_local_dev_build() {
        return None;
    }

    current_project_root().map(|project_root| {
        resolved_project_root(project_root, RuntimeProjectRootMode::DevWorktreeFallback)
    })
}

pub fn resolve_runtime_project_root_path() -> Option<PathBuf> {
    resolve_runtime_project_root().map(|resolved| resolved.path)
}

pub fn resolve_runtime_project_root_or_cwd_fallback() -> ResolvedRuntimeProjectRoot {
    resolve_runtime_project_root().unwrap_or_else(|| {
        let fallback = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        resolved_project_root(fallback, RuntimeProjectRootMode::CurrentDirectoryFallback)
    })
}

pub fn load_runtime_registry(explicit_path: Option<&Path>) -> Option<serde_json::Value> {
    let path = resolve_runtime_registry(explicit_path).ok()?;
    let content = std::fs::read_to_string(path.path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn resolve_runtime_registry(
    explicit_path: Option<&Path>,
) -> Result<ResolvedRuntimeRegistry, String> {
    if let Some(path) = explicit_path {
        return Ok(resolved(
            path.to_path_buf(),
            RuntimeRegistryMode::ExplicitPath,
        ));
    }

    for env_var in ["CYNIC_REGISTRY", "CYNIC_REGISTRY_PATH"] {
        if let Ok(path) = env::var(env_var) {
            return Ok(resolved(
                PathBuf::from(path),
                RuntimeRegistryMode::Environment,
            ));
        }
    }

    let installed_registry = PathBuf::from(DEFAULT_REGISTRY_PATH);
    if installed_registry.is_file() {
        return Ok(resolved(
            installed_registry,
            RuntimeRegistryMode::InstalledAuthority,
        ));
    }

    if allow_local_dev_build()
        && let Some(project_root) = current_project_root()
    {
        let candidate = project_root.join("infra/registry.json");
        if candidate.is_file() {
            return Ok(resolved(
                candidate,
                RuntimeRegistryMode::DevWorktreeFallback,
            ));
        }
    }

    Err(
        "registry path required: set CYNIC_REGISTRY/CYNIC_REGISTRY_PATH, install /etc/cynic/registry.json, or set CYNIC_ALLOW_LOCAL_DEV_BUILD=1 when resolving from a workspace"
            .to_string(),
    )
}

fn allow_local_dev_build() -> bool {
    env::var(ALLOW_LOCAL_DEV_BUILD_ENV)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

fn resolved(path: PathBuf, mode: RuntimeRegistryMode) -> ResolvedRuntimeRegistry {
    ResolvedRuntimeRegistry {
        path,
        mode,
        projection_class: mode.projection_class(),
    }
}

fn resolved_project_root(
    path: PathBuf,
    mode: RuntimeProjectRootMode,
) -> ResolvedRuntimeProjectRoot {
    ResolvedRuntimeProjectRoot {
        path,
        mode,
        projection_class: mode.projection_class(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeProjectRootMode, RuntimeRegistryMode, resolve_runtime_project_root_or_cwd_fallback,
        resolve_runtime_registry,
    };
    use crate::runtime_authority::RuntimeProjectionClass;
    use std::path::Path;

    #[test]
    fn explicit_registry_is_operator_projection() {
        let resolved = resolve_runtime_registry(Some(Path::new("/tmp/registry.json")))
            .expect("explicit registry resolves");
        assert_eq!(resolved.mode, RuntimeRegistryMode::ExplicitPath);
        assert_eq!(
            resolved.projection_class,
            RuntimeProjectionClass::OperatorProjection
        );
    }

    #[test]
    fn current_directory_project_root_fallback_is_not_runtime_authority() {
        let resolved = resolve_runtime_project_root_or_cwd_fallback();
        if resolved.mode == RuntimeProjectRootMode::CurrentDirectoryFallback {
            assert_eq!(
                resolved.projection_class,
                RuntimeProjectionClass::DevWorktreeFallback
            );
        }
    }
}
