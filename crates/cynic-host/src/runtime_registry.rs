use std::env;
use std::path::{Path, PathBuf};

use crate::repo_access::current_project_root;
use crate::runtime_authority::RuntimeProjectionClass;

pub const DEFAULT_REGISTRY_PATH: &str = "/etc/cynic/registry.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRegistryMode {
    ExplicitPath,
    Environment,
    InstalledAuthority,
    ProjectRootEnv,
    DevWorktreeFallback,
}

impl RuntimeRegistryMode {
    pub fn projection_class(self) -> RuntimeProjectionClass {
        match self {
            Self::InstalledAuthority => RuntimeProjectionClass::RuntimeAuthority,
            Self::ExplicitPath | Self::Environment | Self::ProjectRootEnv => {
                RuntimeProjectionClass::OperatorProjection
            }
            Self::DevWorktreeFallback => RuntimeProjectionClass::DevWorktreeFallback,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitPath => "explicit_path",
            Self::Environment => "environment",
            Self::InstalledAuthority => "installed_authority",
            Self::ProjectRootEnv => "project_root_env",
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
    ProjectRootEnv,
    DevWorktreeFallback,
    CurrentDirectoryFallback,
}

impl RuntimeProjectRootMode {
    pub fn projection_class(self) -> RuntimeProjectionClass {
        match self {
            Self::ProjectRootEnv => RuntimeProjectionClass::OperatorProjection,
            Self::DevWorktreeFallback | Self::CurrentDirectoryFallback => {
                RuntimeProjectionClass::DevWorktreeFallback
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProjectRootEnv => "project_root_env",
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
    if let Ok(project_root) = env::var("CYNIC_PROJECT_ROOT") {
        return Some(resolved_project_root(
            PathBuf::from(project_root),
            RuntimeProjectRootMode::ProjectRootEnv,
        ));
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

    if let Ok(project_root) = env::var("CYNIC_PROJECT_ROOT") {
        let candidate = PathBuf::from(project_root).join("infra/registry.json");
        if candidate.is_file() {
            return Ok(resolved(candidate, RuntimeRegistryMode::ProjectRootEnv));
        }
    }

    if let Some(project_root) = current_project_root() {
        let candidate = project_root.join("infra/registry.json");
        if candidate.is_file() {
            return Ok(resolved(
                candidate,
                RuntimeRegistryMode::DevWorktreeFallback,
            ));
        }
    }

    Err(
        "registry path required: set CYNIC_REGISTRY/CYNIC_REGISTRY_PATH, install /etc/cynic/registry.json, or run from a valid CYNIC worktree"
            .to_string(),
    )
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
