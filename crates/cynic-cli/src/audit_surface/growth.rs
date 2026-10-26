//! `cynic audit growth`: projection of the engine's growth report.
//!
//! Informational by design: the exit code is always success. A number becomes
//! a gate only after its starting value has been observed.

use std::process::ExitCode;

use cynic_audit_engine::growth::{GrowthReport, growth_report};
use cynic_audit_engine::scanner::scan_workspace;

use crate::emit_json;

pub(crate) fn execute_growth(json: bool) -> ExitCode {
    let index = scan_workspace(&crate::rust_source::workspace_root());
    let report = growth_report(&index);
    if json {
        return emit_json(&report, false);
    }
    print!("{}", render_text(&report));
    ExitCode::SUCCESS
}

fn render_text(report: &GrowthReport) -> String {
    let mut out = String::new();
    for layer in &report.layers {
        out.push_str(&format!(
            "\n{}  src {}  tests {}\n",
            layer.layer, layer.src_lines, layer.test_lines
        ));
        out.push_str(&format!(
            "  {:<24} {:>7} {:>7} {:>6} {:>6} {:>6} {:>6} {:>6}\n",
            "crate", "src", "tests", ">1000", "fn>100", "dead", "inv", "dupe"
        ));
        for c in &layer.crates {
            out.push_str(&format!(
                "  {:<24} {:>7} {:>7} {:>6} {:>6} {:>6} {:>6} {:>6}\n",
                c.name,
                c.src_lines,
                c.test_lines,
                c.files_over_1000,
                c.fns_over_100,
                c.dead_code_allows,
                c.declared_inversions,
                c.duplicate_type_names.len()
            ));
        }
    }
    out
}
