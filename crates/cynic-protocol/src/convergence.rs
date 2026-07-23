use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CONVERGENCE_MATRIX_SCHEMA: &str = "cynic.convergence_matrix.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvergenceStatus {
    Complete,
    Partial,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConvergenceCell {
    pub subject_id: String,
    pub vertical_id: String,
    pub status: ConvergenceStatus,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConvergenceMatrixReport {
    pub schema: String,
    pub status: ConvergenceStatus,
    pub subjects: Vec<String>,
    pub verticals: Vec<String>,
    pub cells: Vec<ConvergenceCell>,
    pub residual_unknowns: Vec<String>,
    pub blockers: Vec<String>,
}

pub fn assess_convergence_matrix(
    subjects: Vec<String>,
    verticals: Vec<String>,
    cells: Vec<ConvergenceCell>,
) -> ConvergenceMatrixReport {
    let mut residual_unknowns = Vec::new();
    let mut blockers = Vec::new();
    for cell in &cells {
        let key = format!("{}:{}", cell.subject_id, cell.vertical_id);
        match cell.status {
            ConvergenceStatus::Unknown => residual_unknowns.push(key),
            ConvergenceStatus::Blocked => {
                blockers.extend(cell.blockers.iter().map(|value| format!("{key}:{value}")))
            }
            ConvergenceStatus::Complete | ConvergenceStatus::Partial => {}
        }
    }
    let expected = subjects.len().saturating_mul(verticals.len());
    if cells.len() < expected {
        blockers.push("matrix_cells_missing".to_string());
    }
    let status = if !blockers.is_empty() {
        ConvergenceStatus::Blocked
    } else if cells.iter().any(|cell| cell.status == ConvergenceStatus::Unknown) {
        ConvergenceStatus::Unknown
    } else if cells.iter().all(|cell| cell.status == ConvergenceStatus::Complete)
        && cells.len() >= expected
    {
        ConvergenceStatus::Complete
    } else {
        ConvergenceStatus::Partial
    };
    ConvergenceMatrixReport {
        schema: CONVERGENCE_MATRIX_SCHEMA.to_string(),
        status,
        subjects,
        verticals,
        cells,
        residual_unknowns,
        blockers,
    }
}

pub fn merge_convergence_matrices(
    matrices: &[ConvergenceMatrixReport],
) -> ConvergenceMatrixReport {
    let mut subjects = BTreeSet::new();
    let mut verticals = BTreeSet::new();
    let mut cells = BTreeMap::new();
    let mut blockers = Vec::new();

    for matrix in matrices {
        subjects.extend(matrix.subjects.iter().cloned());
        verticals.extend(matrix.verticals.iter().cloned());
        blockers.extend(matrix.blockers.iter().map(|blocker| {
            format!("source_matrix:{}:{blocker}", matrix.schema)
        }));
        for cell in &matrix.cells {
            let key = (cell.subject_id.clone(), cell.vertical_id.clone());
            if cells.insert(key.clone(), cell.clone()).is_some() {
                blockers.push(format!("duplicate_cell:{}:{}", key.0, key.1));
            }
        }
    }

    let report = assess_convergence_matrix(
        subjects.into_iter().collect(),
        verticals.into_iter().collect(),
        cells.into_values().collect(),
    );
    if blockers.is_empty() {
        report
    } else {
        ConvergenceMatrixReport {
            status: ConvergenceStatus::Blocked,
            blockers: blockers
                .into_iter()
                .chain(report.blockers)
                .collect(),
            ..report
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_cells_block_matrix_convergence() {
        let report = assess_convergence_matrix(
            vec!["cortex".into(), "proxmox".into()],
            vec!["host_os".into(), "network".into()],
            vec![ConvergenceCell {
                subject_id: "cortex".into(),
                vertical_id: "host_os".into(),
                status: ConvergenceStatus::Complete,
                evidence_refs: vec!["proof-1".into()],
                blockers: Vec::new(),
            }],
        );
        assert_eq!(report.status, ConvergenceStatus::Blocked);
        assert!(report.blockers.contains(&"matrix_cells_missing".to_string()));
    }

    #[test]
    fn unknown_cell_is_not_promoted_to_partial_success() {
        let report = assess_convergence_matrix(
            vec!["cortex".into()],
            vec!["provider".into()],
            vec![ConvergenceCell {
                subject_id: "cortex".into(),
                vertical_id: "provider".into(),
                status: ConvergenceStatus::Unknown,
                evidence_refs: Vec::new(),
                blockers: Vec::new(),
            }],
        );
        assert_eq!(report.status, ConvergenceStatus::Unknown);
        assert_eq!(report.residual_unknowns, vec!["cortex:provider"]);
    }

    #[test]
    fn fleet_merge_requires_distinct_node_cells() {
        let node = |subject_id: &str| ConvergenceMatrixReport {
            schema: CONVERGENCE_MATRIX_SCHEMA.to_string(),
            status: ConvergenceStatus::Complete,
            subjects: vec![subject_id.to_string()],
            verticals: vec!["host_os".to_string()],
            cells: vec![ConvergenceCell {
                subject_id: subject_id.to_string(),
                vertical_id: "host_os".to_string(),
                status: ConvergenceStatus::Complete,
                evidence_refs: vec![format!("{subject_id}-proof")],
                blockers: Vec::new(),
            }],
            residual_unknowns: Vec::new(),
            blockers: Vec::new(),
        };

        let report = merge_convergence_matrices(&[node("cortex"), node("proxmox")]);
        assert_eq!(report.status, ConvergenceStatus::Complete);
        assert_eq!(report.cells.len(), 2);
    }
}
