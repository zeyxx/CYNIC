//! Capacity-aware vault consolidation entry point.
//!
//! Thin wrapper over [`super::vault_consolidate::consolidate_vault`] that
//! honors an optional `capacity_bytes` budget. When `Some` and the observed
//! vault size already exceeds the budget, consolidation is downgraded to a
//! read-only plan and the report records the residual unknown instead of
//! claiming convergence.

use crate::application::vault_consolidate::{VaultConsolidateReport, consolidate_vault};
use crate::ports::local_runtime_port::LocalRuntimeError;

/// Plan or apply vault consolidation, bounded by an optional capacity budget.
///
/// `capacity_bytes`: when `Some(n)`, if the vault path already occupies more
/// than `n` bytes the routine refuses to mutate and returns a read-only report
/// with `status = "capacity_exceeded"`.
pub fn consolidate_vault_with_capacity(
    apply: bool,
    capacity_bytes: Option<u64>,
) -> Result<VaultConsolidateReport, LocalRuntimeError> {
    if let (true, Some(cap), Some(vault)) = (
        apply,
        capacity_bytes,
        std::env::var_os("CYNIC_VAULT_DIR").map(std::path::PathBuf::from),
    ) {
        if let Ok(meta) = std::fs::metadata(&vault) {
            if meta.is_dir() {
                let used = dir_size(&vault).unwrap_or(0);
                if used > cap {
                    let mut report = consolidate_vault(false)?;
                    report.status = "capacity_exceeded";
                    report.applied = false;
                    report.message = format!(
                        "vault already uses {used} bytes, exceeds capacity budget {cap}; mutation refused"
                    );
                    return Ok(report);
                }
            }
        }
    }
    consolidate_vault(apply)
}

fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                total += entry.metadata()?.len();
            }
        }
    }
    Ok(total)
}
