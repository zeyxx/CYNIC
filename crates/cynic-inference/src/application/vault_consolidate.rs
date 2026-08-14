//! Centralize model weights into the principal vault by **physical move**.
//!
//! Scan known spill locations (llama cache, HF cache, operator cynic models,
//! external profile paths). Plan or apply relocation into `ModelLayout.vault_dir`,
//! then rewrite profile `LLAMA_MODEL_PATH` to the new vault path.
//!
//! Mutation requires explicit `apply=true` (`--yes` on CLI).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::adapters::local::FsModelStore;
use crate::domain::model_layout::ModelLayout;
use crate::domain::model_store::{ModelArtifact, ModelLocationKind};
use crate::ports::local_runtime_port::LocalRuntimeError;
use crate::ports::model_store_port::ModelStorePort;

#[derive(Debug, Clone, Serialize)]
pub struct VaultConsolidateAction {
    pub id: String,
    pub from: String,
    pub to: String,
    pub size_bytes: Option<u64>,
    pub source_kind: String,
    pub profile_aliases: Vec<String>,
    pub status: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultConsolidateReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub applied: bool,
    pub vault_dir: String,
    pub discovery_roots: Vec<String>,
    pub actions: Vec<VaultConsolidateAction>,
    pub moved: usize,
    pub skipped: usize,
    pub failed: usize,
    pub profiles_rewritten: usize,
    pub message: String,
    pub rules: Vec<&'static str>,
}

impl VaultConsolidateReport {
    pub const SCHEMA: &'static str = "cynic.inference.vault_consolidate.v1";
}

/// Directories observed for stray weights (in addition to layout vault).
pub fn discovery_roots(layout: &ModelLayout) -> Vec<PathBuf> {
    let mut roots = vec![
        layout.vault_dir.clone(),
        layout.llama_cache_dir.clone(),
        layout.hf_cache_dir.clone(),
    ];
    // Operator spill (legacy) — only if principal vault is elsewhere.
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        roots.push(home.join(".cache/llama.cpp"));
        roots.push(home.join(".local/share/cynic/models"));
        roots.push(home.join(".cache/huggingface/hub"));
    }
    // Explicit extra roots (colon/pathsep-separated), agnostic.
    if let Ok(extra) = std::env::var("CYNIC_MODEL_DISCOVERY_ROOTS") {
        for p in extra.split([':', ';']) {
            let p = p.trim();
            if !p.is_empty() {
                roots.push(PathBuf::from(p));
            }
        }
    }
    roots.sort();
    roots.dedup();
    roots.into_iter().filter(|p| p.is_dir()).collect()
}

/// Plan (apply=false) or execute (apply=true) physical consolidation into vault.
pub fn consolidate_vault(apply: bool) -> Result<VaultConsolidateReport, LocalRuntimeError> {
    let layout = ModelLayout::from_env();
    let roots = discovery_roots(&layout);
    let store = FsModelStore::with_layout(layout.clone());

    // Full scan + profile links
    let mut report = store.scan()?;
    // Augment with discovery roots not already covered by default scan
    for root in &roots {
        if root == &layout.vault_dir
            || root == &layout.llama_cache_dir
            || root == &layout.hf_cache_dir
        {
            continue;
        }
        scan_gguf_into(root, &layout, &mut report.artifacts);
    }
    // Profiles may point external — ensure those paths appear as candidates
    for profile in &report.profiles {
        if profile.model_exists {
            let p = PathBuf::from(&profile.model_path);
            if !report
                .artifacts
                .iter()
                .any(|a| a.path == profile.model_path)
            {
                report.artifacts.push(ModelArtifact {
                    id: profile.alias.clone(),
                    path: profile.model_path.clone(),
                    location: classify_path(&layout, &p),
                    kind: ext_kind(&p),
                    size_bytes: profile.size_bytes,
                    exists: true,
                    profile_alias: Some(profile.alias.clone()),
                    hf_repo: None,
                    hf_revision: None,
                });
            }
        }
    }

    let mut actions = Vec::new();
    for art in &report.artifacts {
        if !art.exists || art.kind != "gguf" {
            continue;
        }
        if art.location == ModelLocationKind::Vault {
            // Already in vault — still rewrite profiles if they point outside but same file?
            continue;
        }
        let from = PathBuf::from(&art.path);
        if !from.is_file() {
            continue;
        }
        // Skip if already under vault after canonicalize
        if is_under(&from, &layout.vault_dir) {
            continue;
        }
        // Skip hardlinks that already share an inode with a vault file (no physical work).
        if let Some(name) = from.file_name() {
            let vault_peer = layout.vault_dir.join(name);
            if vault_peer.is_file() && same_file(&from, &vault_peer) {
                continue;
            }
        }

        let to = unique_dest(&layout.vault_dir, &from)?;
        let aliases: Vec<String> = report
            .profiles
            .iter()
            .filter(|p| p.model_path == art.path)
            .map(|p| p.alias.clone())
            .collect();
        // Also profiles that resolve to same file via alias linkage
        let mut aliases = aliases;
        if let Some(a) = &art.profile_alias
            && !aliases.contains(a)
        {
            aliases.push(a.clone());
        }

        actions.push(VaultConsolidateAction {
            id: art.id.clone(),
            from: from.display().to_string(),
            to: to.display().to_string(),
            size_bytes: art.size_bytes,
            source_kind: format!("{:?}", art.location).to_ascii_lowercase(),
            profile_aliases: aliases,
            status: if apply { "pending" } else { "planned" },
            detail: if apply {
                "will move".into()
            } else {
                "plan only — pass --yes to move into vault".into()
            },
        });
    }

    let mut moved = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut profiles_rewritten = 0usize;

    if apply {
        fs::create_dir_all(&layout.vault_dir).map_err(|e| {
            LocalRuntimeError::Control(format!("create vault {}: {e}", layout.vault_dir.display()))
        })?;
        for action in &mut actions {
            match physical_move(Path::new(&action.from), Path::new(&action.to)) {
                Ok(how) => {
                    action.status = "moved";
                    action.detail = how;
                    moved += 1;
                    // rewrite profiles pointing at old path
                    let n = rewrite_profiles_model_path(
                        &layout,
                        &action.from,
                        &action.to,
                        &action.profile_aliases,
                    )?;
                    profiles_rewritten += n;
                }
                Err(e) => {
                    action.status = "failed";
                    action.detail = e;
                    failed += 1;
                }
            }
        }
        // Refresh registry best-effort
        if let Ok(fresh) = store.scan() {
            let _ = store.persist_registry(&fresh);
        }
    } else {
        skipped = actions.len();
    }

    let status = if !apply {
        "planned"
    } else if failed == 0 {
        "ok"
    } else if moved > 0 {
        "degraded"
    } else {
        "error"
    };

    let n_actions = actions.len();
    let message = if apply {
        format!(
            "moved={moved} failed={failed} profiles_rewritten={profiles_rewritten} vault={}",
            layout.vault_dir.display()
        )
    } else {
        format!("{n_actions} candidate(s) outside vault — re-run with --yes to move")
    };

    Ok(VaultConsolidateReport {
        schema: VaultConsolidateReport::SCHEMA,
        status,
        mutates: apply,
        applied: apply,
        vault_dir: layout.vault_dir.display().to_string(),
        discovery_roots: roots.iter().map(|p| p.display().to_string()).collect(),
        actions,
        moved,
        skipped: if apply { 0 } else { skipped },
        failed,
        profiles_rewritten,
        message,
        rules: vec![
            "vault is the only durable SSOT for native startable weights",
            "physical move (rename same FS, else copy+remove)",
            "profiles LLAMA_MODEL_PATH are rewritten to vault paths",
            "requires explicit --yes; plan is default",
            "CYNIC_MODEL_DISCOVERY_ROOTS adds extra scan roots (pathsep : or ;)",
            "HF hub blobs may be large — only GGUF files are candidates",
        ],
    })
}

fn scan_gguf_into(dir: &Path, layout: &ModelLayout, out: &mut Vec<ModelArtifact>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_gguf_into(&path, layout, out);
            continue;
        }
        if ext_kind(&path) != "gguf" {
            continue;
        }
        let path_s = path.display().to_string();
        if out.iter().any(|a| a.path == path_s) {
            continue;
        }
        let meta = fs::metadata(&path).ok();
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        out.push(ModelArtifact {
            id,
            path: path_s,
            location: classify_path(layout, &path),
            kind: "gguf".into(),
            size_bytes: meta.map(|m| m.len()),
            exists: true,
            profile_alias: None,
            hf_repo: None,
            hf_revision: None,
        });
    }
}

fn classify_path(layout: &ModelLayout, path: &Path) -> ModelLocationKind {
    let p = path.to_string_lossy();
    if p.starts_with(layout.vault_dir.to_string_lossy().as_ref()) {
        ModelLocationKind::Vault
    } else if p.starts_with(layout.hf_cache_dir.to_string_lossy().as_ref()) {
        ModelLocationKind::HfCache
    } else if p.starts_with(layout.llama_cache_dir.to_string_lossy().as_ref()) {
        ModelLocationKind::LlamaCache
    } else {
        ModelLocationKind::External
    }
}

fn ext_kind(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_under(path: &Path, root: &Path) -> bool {
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    path.starts_with(root)
}

fn unique_dest(vault: &Path, from: &Path) -> Result<PathBuf, LocalRuntimeError> {
    let name = from
        .file_name()
        .ok_or_else(|| LocalRuntimeError::Control("source has no file name".into()))?;
    let mut dest = vault.join(name);
    if !dest.exists() {
        return Ok(dest);
    }
    // Same file already?
    if same_file(from, &dest) {
        return Ok(dest);
    }
    let stem = from.file_stem().and_then(|s| s.to_str()).unwrap_or("model");
    let ext = from.extension().and_then(|s| s.to_str()).unwrap_or("gguf");
    for n in 2u32..1000 {
        dest = vault.join(format!("{stem}-{n}.{ext}"));
        if !dest.exists() {
            return Ok(dest);
        }
    }
    Err(LocalRuntimeError::Control(
        "could not allocate unique vault destination".into(),
    ))
}

fn same_file(a: &Path, b: &Path) -> bool {
    let (Ok(ma), Ok(mb)) = (fs::metadata(a), fs::metadata(b)) else {
        return false;
    };
    if ma.len() != mb.len() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ma.dev() == mb.dev() && ma.ino() == mb.ino()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn physical_move(from: &Path, to: &Path) -> Result<String, String> {
    if same_file(from, to) {
        return Ok("already at vault path (same inode)".into());
    }
    if to.exists() {
        return Err(format!("destination exists: {}", to.display()));
    }
    // Prefer rename (same filesystem); fallback copy+remove.
    match fs::rename(from, to) {
        Ok(()) => Ok("renamed into vault".into()),
        Err(rename_err) => {
            fs::copy(from, to)
                .map_err(|e| format!("copy to vault: {e} (rename was: {rename_err})"))?;
            fs::remove_file(from).map_err(|e| {
                format!(
                    "copied to vault but failed to remove source {}: {e}",
                    from.display()
                )
            })?;
            Ok("copied into vault and removed source".into())
        }
    }
}

fn rewrite_profiles_model_path(
    layout: &ModelLayout,
    old: &str,
    new: &str,
    aliases: &[String],
) -> Result<usize, LocalRuntimeError> {
    let mut n = 0usize;
    let mut targets: Vec<PathBuf> = aliases
        .iter()
        .map(|a| layout.profiles_dir.join(format!("{a}.env")))
        .collect();
    // Also rewrite any profile that still mentions old path
    if let Ok(rd) = fs::read_dir(&layout.profiles_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("env") {
                targets.push(p);
            }
        }
    }
    targets.sort();
    targets.dedup();
    for path in targets {
        if !path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .map_err(|e| LocalRuntimeError::Inventory(format!("read {}: {e}", path.display())))?;
        if !raw.contains(old) {
            continue;
        }
        let updated = raw.replace(old, new);
        fs::write(&path, updated)
            .map_err(|e| LocalRuntimeError::Inventory(format!("write {}: {e}", path.display())))?;
        n += 1;
    }
    // runtime env copies
    let runtime_dir = layout.profiles_dir.join("runtime");
    if runtime_dir.is_dir()
        && let Ok(rd) = fs::read_dir(&runtime_dir)
    {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("env") {
                continue;
            }
            let Ok(raw) = fs::read_to_string(&p) else {
                continue;
            };
            if !raw.contains(old) {
                continue;
            }
            let _ = fs::write(&p, raw.replace(old, new));
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_dest_avoids_collision() {
        let root = std::env::temp_dir().join(format!("cynic-vault-uniq-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let from = root.join("src").join("m.gguf");
        fs::create_dir_all(from.parent().unwrap()).unwrap();
        fs::write(&from, b"abc").unwrap();
        let vault = root.join("vault");
        fs::create_dir_all(&vault).unwrap();
        fs::write(vault.join("m.gguf"), b"other").unwrap();
        let dest = unique_dest(&vault, &from).unwrap();
        assert!(dest.file_name().unwrap().to_string_lossy().contains("m-2"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn physical_move_rename() {
        let root = std::env::temp_dir().join(format!("cynic-vault-mv-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let from = root.join("a.gguf");
        let to = root.join("b.gguf");
        fs::create_dir_all(&root).unwrap();
        fs::write(&from, b"gguf-data").unwrap();
        physical_move(&from, &to).unwrap();
        assert!(!from.exists());
        assert_eq!(fs::read(&to).unwrap(), b"gguf-data");
        let _ = fs::remove_dir_all(&root);
    }
}
