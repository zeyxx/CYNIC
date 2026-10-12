//! Build proofs listing and metabolic invoicing.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum BuildProofsCommand {
    /// List pipeline proofs, most recent first.
    List {
        /// Keep only proofs for this exact source commit.
        #[arg(long)]
        commit: Option<String>,
        /// Keep at most N newest proofs (0 = no limit).
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Proof log path. Defaults to the operator log under $HOME.
        #[arg(long)]
        proof_log: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn list_proofs(
    commit: Option<&str>,
    limit: usize,
    proof_log: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let log_path = match proof_log {
        Some(explicit) => explicit.to_string(),
        None => crate::build_proof_log::default_proof_log_path()?,
    };
    let (proofs, skipped) = crate::build_proof_log::read_proofs(&log_path, commit, limit)?;
    if json {
        println!(
            "{}",
            serde_json::json!({ "proof_log": log_path, "proofs": proofs, "skipped_lines": skipped })
        );
    } else {
        println!("proof log {log_path}: {} retained", proofs.len());
        for proof in &proofs {
            println!("  {}", proof);
        }
        for skipped in &skipped {
            println!("  skipped: {skipped}");
        }
    }
    Ok(())
}

pub(crate) fn invoice_for_commit(
    commit: &str,
    proof_log: Option<&str>,
    invoice_log: Option<&str>,
    json: bool,
) -> Result<(), String> {
    use cynic_node::ports::build_runner::{BuildProofBundle, invoice_from_proofs};

    let log_path = match proof_log {
        Some(explicit) => explicit.to_string(),
        None => crate::build_proof_log::default_proof_log_path()?,
    };
    let (proofs, skipped) = crate::build_proof_log::read_proofs(&log_path, Some(commit), 0)?;
    let mut bundles = Vec::new();
    let mut legacy = 0usize;
    for proof in &proofs {
        match serde_json::from_value::<BuildProofBundle>(proof.clone()) {
            Ok(bundle) => bundles.push(bundle),
            Err(_) => legacy += 1,
        }
    }
    let mut invoice = invoice_from_proofs(commit, &bundles);
    invoice.proofs_observed = proofs.len();
    if proofs.is_empty() {
        invoice
            .residual_unknowns
            .push("no proofs observed for commit".to_string());
    }
    for line in &skipped {
        invoice.residual_unknowns.push(line.clone());
    }
    if legacy > 0 {
        invoice.residual_unknowns.push(format!(
            "{legacy} proof(s) predate duration_ms and carry no observed wall time"
        ));
    }
    invoice
        .validate()
        .map_err(|blockers| format!("invoice invalid: {}", blockers.join(", ")))?;
    let rendered =
        serde_json::to_string_pretty(&invoice).map_err(|e| format!("serialize invoice: {e}"))?;
    if json {
        println!("{rendered}");
    } else {
        println!(
            "metabolic invoice {commit}: {} proofs invoiced, {} ms wall",
            invoice.proofs_invoiced, invoice.total.wall_time_ms
        );
        for unknown in &invoice.residual_unknowns {
            println!("  unknown: {unknown}");
        }
    }
    let journal_path = match invoice_log {
        Some(explicit) => explicit.to_string(),
        None => crate::invoice_log::default_invoice_log_path()?,
    };
    if let Err(error) = crate::invoice_log::append_invoice(&journal_path, &rendered) {
        eprintln!("warning: invoice retained in stdout only: {error}");
    }
    Ok(())
}
