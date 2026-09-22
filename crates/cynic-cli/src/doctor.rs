use serde::Serialize;
use std::path::Path;

/// Doctor report — scan of the codebase against the 7 design rules.
#[derive(Serialize)]
pub struct DoctorReport {
    pub schema: &'static str,
    pub score: f64,
    pub violations: Vec<Violation>,
    pub summary: Summary,
}

#[derive(Serialize)]
pub struct Violation {
    pub rule: &'static str,
    pub severity: Severity,
    pub count: usize,
    pub files: Vec<String>,
    pub recommendation: String,
}

#[derive(Serialize)]
pub enum Severity {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}

#[derive(Serialize)]
pub struct Summary {
    pub total_files: usize,
    pub total_lines: usize,
    pub violations_high: usize,
    pub violations_medium: usize,
    pub violations_low: usize,
}

/// Scan the codebase against the 7 design rules.
pub fn run_doctor(repo_root: &Path) -> Result<DoctorReport, String> {
    let mut violations: Vec<Violation> = Vec::new();
    let mut total_files: usize = 0;
    let mut total_lines: usize = 0;

    // R1: Files > 400 lines
    let r1 = scan_file_lengths(repo_root, 400)?;
    total_files = r1.0;
    total_lines = r1.1;
    if !r1.2.is_empty() {
        violations.push(Violation {
            rule: "R1",
            severity: Severity::Medium,
            count: r1.2.len(),
            files: r1
                .2
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            recommendation: format!(
                "Split files >400 lines ({} found). Use natural seams.",
                r1.2.len()
            ),
        });
    }

    // R2: CLI imports directly from host/node/inference
    let r2 = scan_cli_coupling(repo_root)?;
    if !r2.is_empty() {
        violations.push(Violation {
            rule: "R2",
            severity: Severity::High,
            count: r2.len(),
            files: r2.iter().map(|p| p.to_string_lossy().to_string()).collect(),
            recommendation: format!(
                "CLI imports directly from host/node/inference ({} found). Use HostPort, NodePort, InferencePort traits in protocol.",
                r2.len()
            ),
        });
    }

    // R4: Unbounded loops
    let r4 = scan_unbounded_loops(repo_root)?;
    if !r4.is_empty() {
        violations.push(Violation {
            rule: "R4",
            severity: Severity::Medium,
            count: r4.len(),
            files: r4.iter().map(|p| p.to_string_lossy().to_string()).collect(),
            recommendation: format!(
                "Unbounded loops detected ({} found). Add max_iterations.",
                r4.len()
            ),
        });
    }

    // R5: println without read-back
    let r5 = scan_println_without_proof(repo_root)?;
    if !r5.is_empty() {
        violations.push(Violation {
            rule: "R5",
            severity: Severity::Low,
            count: r5.len(),
            files: r5.iter().map(|p| p.to_string_lossy().to_string()).collect(),
            recommendation: format!(
                "println! without read-back proof ({} found). Use ReadBackProof.",
                r5.len()
            ),
        });
    }

    let violations_high = violations
        .iter()
        .filter(|v| matches!(v.severity, Severity::High))
        .count();
    let violations_medium = violations
        .iter()
        .filter(|v| matches!(v.severity, Severity::Medium))
        .count();
    let violations_low = violations
        .iter()
        .filter(|v| matches!(v.severity, Severity::Low))
        .count();

    // Score: 1.0 - (weighted penalties)
    let score = 1.0
        - (violations_high as f64 * 0.15)
        - (violations_medium as f64 * 0.05)
        - (violations_low as f64 * 0.02);

    Ok(DoctorReport {
        schema: "cynic.dev.doctor.v1",
        score: score.max(0.0),
        violations,
        summary: Summary {
            total_files,
            total_lines,
            violations_high,
            violations_medium,
            violations_low,
        },
    })
}

/// R1: Scan for files over `max_lines`.
fn scan_file_lengths(
    repo_root: &Path,
    max_lines: usize,
) -> Result<(usize, usize, Vec<std::path::PathBuf>), String> {
    let mut files_over: Vec<std::path::PathBuf> = Vec::new();
    let mut total_files: usize = 0;
    let mut total_lines: usize = 0;

    let src_dir = repo_root.join("crates");
    if !src_dir.exists() {
        return Ok((0, 0, files_over));
    }

    for entry in walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "rs")
        })
    {
        total_files += 1;
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            let lines = content.lines().count();
            total_lines += lines;
            if lines > max_lines {
                files_over.push(entry.path().to_path_buf());
            }
        }
    }

    Ok((total_files, total_lines, files_over))
}

/// R2: Scan for CLI files importing directly from host/node/inference.
fn scan_cli_coupling(repo_root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut offenders: Vec<std::path::PathBuf> = Vec::new();

    let cli_dir = repo_root.join("crates/cynic-cli/src");
    if !cli_dir.exists() {
        return Ok(offenders);
    }

    for entry in walkdir::WalkDir::new(&cli_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "rs")
        })
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if content.contains("use cynic_host::")
                || content.contains("use cynic_node::")
                || content.contains("use cynic_inference::")
            {
                offenders.push(entry.path().to_path_buf());
            }
        }
    }

    Ok(offenders)
}

/// R4: Scan for unbounded loops (loop without max_iterations).
fn scan_unbounded_loops(repo_root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut offenders: Vec<std::path::PathBuf> = Vec::new();

    let src_dir = repo_root.join("crates");
    if !src_dir.exists() {
        return Ok(offenders);
    }

    for entry in walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "rs")
        })
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if content.contains("loop") && !content.contains("max_iterations") {
                offenders.push(entry.path().to_path_buf());
            }
        }
    }

    Ok(offenders)
}

/// R5: Scan for println without read-back proof.
fn scan_println_without_proof(repo_root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut offenders: Vec<std::path::PathBuf> = Vec::new();

    let cli_dir = repo_root.join("crates/cynic-cli/src");
    if !cli_dir.exists() {
        return Ok(offenders);
    }

    for entry in walkdir::WalkDir::new(&cli_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "rs")
        })
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if content.contains("println!")
                && !content.contains("read_back")
                && !content.contains("ReadBackProof")
            {
                offenders.push(entry.path().to_path_buf());
            }
        }
    }

    Ok(offenders)
}
