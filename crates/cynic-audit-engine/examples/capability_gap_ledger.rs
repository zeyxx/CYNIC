//! Generate `infra/capability-gap-ledger.json` from a live observation of the
//! workspace.
//!
//! Every direct `std::process::Command::new` (or `tokio::process::Command::new`)
//! call site is a process-spawn surface that bypasses the capability membrane:
//! the program is chosen in source rather than admitted through the declared
//! capability catalog. This writer records them so the gap can only shrink.
//!
//! Usage: cargo run -p cynic-audit-engine --example capability_gap_ledger -- <root>
//!
//! The output is deterministic: surfaces are sorted by path then line, object
//! keys are emitted in a fixed order, and the summary aggregates by risk class.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use cynic_audit_engine::scanner::scan_workspace;

#[derive(Debug, Clone)]
struct Surface {
    path: String,
    line: usize,
    program: String,
    risk_class: String,
    is_capability_binary: bool,
    /// A spawn inside a test module is an assertion, not a capability.
    is_test: bool,
}

/// Extract the program argument of a `Command::new(...)` call.
///
/// Returns the literal string when the argument is a string literal, and
/// `"dynamic"` when the program is computed at runtime. Returns `None` when
/// the line does not contain such a call.
fn extract_program(line: &str) -> Option<String> {
    let marker = "Command::new(";
    let start = line.find(marker)?;
    let after = &line[start + marker.len()..];
    // Balance parentheses so a nested call (e.g. env!(...)) is captured whole.
    let mut depth = 1usize;
    let mut end = 0usize;
    for (i, c) in after.chars().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    let arg = after[..end].trim();
    if arg.starts_with('"') && arg.ends_with('"') && arg.len() >= 2 {
        // Strip the surrounding quotes; keep the inner text verbatim.
        Some(arg[1..arg.len() - 1].to_string())
    } else {
        Some("dynamic".to_string())
    }
}

fn strip_string_literals(line: &str) -> String {
    let mut out = String::new();
    let mut in_str = false;
    let mut escape = false;
    for c in line.chars() {
        if in_str {
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else {
            out.push(c);
        }
    }
    out
}

fn relative_to_workspace(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn classify(program: &str, _relative_path: &str) -> &'static str {
    if program == "dynamic" {
        return "self-exec";
    }
    let base = Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program)
        .to_ascii_lowercase();
    match base.as_str() {
        // Local build: a compilation tool. This is the class the
        // deny-by-default policy refuses outside a build runner.
        "cargo" | "rustc" | "rustup" | "make" | "cmake" | "ninja" | "cc" | "gcc" | "ld" => {
            "local-build"
        }
        // Remote execution: crosses a host or container boundary.
        "ssh" | "ssh-keyscan" | "scp" | "rsync" | "sftp" | "sudo" => "remote-exec",
        // Package managers.
        "apt" | "apt-get" | "dpkg" | "npm" | "pnpm" | "yarn" | "pip" | "uv" => {
            "package-manager"
        }
        // Provider-specific CLIs.
        "curl" | "wget" | "forgejo" | "gh" | "hermes" | "docker" | "podman" | "kubectl"
        | "bwrap" | "pct" | "qemu" => "provider-specific",
        // Observability / read-only inspection.
        "systemctl" | "journalctl" | "which" | "getent" | "id" | "ping" | "dmesg" | "wg"
        | "grep" | "mkfifo" | "systemd-creds" | "kill" | "ssh-keygen" | "df" | "free"
        | "ps" | "uptime" | "stat" => "observability",
        // Version control: a transport, not a build.
        "git" | "git-lfs" => "vcs",
        // Host mutation tooling: changes the machine.
        "chown" | "chmod" | "ln" | "mount" | "umount" | "useradd" | "usermod" | "tar" => {
            "host-mutation"
        }
        // Everything else is unclassified: naming it honestly beats
        // filing it under a class that hides it.
        _ => "unclassified",
    }
}

fn capability_binaries(root: &Path) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let text = fs::read_to_string(root.join("infra/capabilities/catalog.json")).unwrap_or_default();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(caps) = value.get("capabilities").and_then(|v| v.as_array()) {
            for cap in caps {
                if let Some(argv) = cap.get("argv_template").and_then(|v| v.as_array()) {
                    if let Some(first) = argv.first().and_then(|v| v.as_str()) {
                        set.insert(first.to_string());
                        if let Some(name) = Path::new(first).file_name().and_then(|n| n.to_str()) {
                            set.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }
    set
}

fn is_capability_binary(program: &str, catalog: &BTreeSet<String>) -> bool {
    if program == "dynamic" {
        return false;
    }
    if catalog.contains(program) {
        return true;
    }
    let base = Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);
    catalog.contains(base)
}

/// Whether a line belongs to a test module rather than production code.
///
/// A test asserts behaviour; it does not grant a capability. A spawn in a
/// test is therefore not a surface that bypasses the membrane, and
/// counting it as one would make the ledger report a risk that does not
/// exist.
fn is_test_surface(relative_path: &str, lines: &[&str], line_index: usize) -> bool {
    // A file under a `tests/` directory is an integration test.
    if relative_path.contains("/tests/") || relative_path.starts_with("tests/") {
        return true;
    }
    // A `#[cfg(test)] mod tests` block: look backwards for the module
    // header that opens the test section of the file.
    for previous in lines[..line_index].iter().rev() {
        let trimmed = previous.trim();
        if trimmed.starts_with("#[cfg(test)]") {
            return true;
        }
        // A new top-level item ends the test section.
        if trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") {
            return false;
        }
    }
    false
}

fn sort_surfaces(surfaces: &mut [Surface]) {
    surfaces.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
}

fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let index = scan_workspace(&root);
    let catalog = capability_binaries(&root);

    let mut surfaces: Vec<Surface> = Vec::new();
    for file in &index.rust_files {
        for (i, line) in file.content.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if !strip_string_literals(line).contains("Command::new") {
                continue;
            }
            let program = extract_program(line).unwrap_or_else(|| "dynamic".to_string());
            let risk = classify(&program, &file.relative_path);
            let is_cap = is_capability_binary(&program, &catalog);
            let all_lines: Vec<&str> = file.content.lines().collect();
            let is_test = is_test_surface(&file.relative_path, &all_lines, i);
            surfaces.push(Surface {
                path: file.relative_path.clone(),
                line: i + 1,
                program,
                risk_class: risk.to_string(),
                is_capability_binary: is_cap,
                is_test,
            });
        }
    }
    sort_surfaces(&mut surfaces);

    // Summary: counts per risk class (all six classes, even when zero).
    let classes = [
        "local-build",
        "observability",
        "package-manager",
        "provider-specific",
        "remote-exec",
        "self-exec",
    ];
    let bypass_count = surfaces
        .iter()
        .filter(|s| s.is_capability_binary && !s.is_test)
        .count();
    let production_count = surfaces.iter().filter(|s| !s.is_test).count();
    let test_count = surfaces.len() - production_count;
    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    for class in classes {
        by_class.insert(class.to_string(), 0);
    }
    for surface in surfaces.iter().filter(|s| !s.is_test) {
        *by_class.entry(surface.risk_class.clone()).or_insert(0) += 1;
    }

    let mut document = String::new();
    document.push_str("{\n");
    document.push_str("  \"schema\": \"cynic.capability_gap_ledger.v1\",\n");
    document.push_str("  \"surfaces\": [\n");
    let mut surface_lines: Vec<String> = Vec::new();
    for s in &surfaces {
        surface_lines.push(format!(
            "    {{\"path\": \"{}\", \"line\": {}, \"program\": \"{}\", \"risk_class\": \"{}\", \"is_capability_binary\": {}, \"is_test\": {}}}",
            escape_json(&s.path),
            s.line,
            escape_json(&s.program),
            escape_json(&s.risk_class),
            s.is_capability_binary,
            s.is_test
        ));
    }
    document.push_str(&surface_lines.join(",\n"));
    document.push_str("\n  ],\n");
    document.push_str("  \"summary\": {\n");
    document.push_str("    \"by_risk_class\": {\n");
    let class_lines: Vec<String> = by_class
        .iter()
        .map(|(k, v)| format!("      \"{}\": {}", escape_json(k), v))
        .collect();
    document.push_str(&class_lines.join(",\n"));
    document.push_str("\n    },\n");
    document.push_str(&format!(
        "    \"capability_bypass_count\": {},\n",
        bypass_count
    ));
    document.push_str(&format!(
        "    \"production_surface_count\": {},\n",
        production_count
    ));
    document.push_str(&format!("    \"test_surface_count\": {}\n", test_count));
    document.push_str("  }\n");
    document.push_str("}\n");

    let out = root.join("infra/capability-gap-ledger.json");
    fs::write(&out, document).expect("write the capability gap ledger");
    println!(
        "wrote {} with {} production surfaces ({} test surfaces) and {} capability bypasses",
        out.display(),
        production_count,
        test_count,
        bypass_count
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_literal_vs_dynamic() {
        assert_eq!(
            extract_program("let x = Command::new(\"git\").arg(\"status\");"),
            Some("git".to_string())
        );
        assert_eq!(
            extract_program("Command::new(program)"),
            Some("dynamic".to_string())
        );
        assert_eq!(
            extract_program("std::process::Command::new(env!(\"CARGO_BIN_EXE_cynic\"))"),
            Some("dynamic".to_string())
        );
    }

    #[test]
    fn classifies_risk() {
        assert_eq!(classify("ssh", "x.rs"), "remote-exec");
        assert_eq!(classify("dynamic", "x.rs"), "self-exec");
        assert_eq!(classify("git", "x.rs"), "local-build");
        assert_eq!(classify("cargo", "x.rs"), "package-manager");
        assert_eq!(classify("curl", "x.rs"), "provider-specific");
        assert_eq!(classify("which", "x.rs"), "observability");
    }

    #[test]
    fn sorts_deterministically() {
        let mut surfaces = vec![
            Surface {
                path: "b.rs".to_string(),
                line: 2,
                program: "x".to_string(),
                risk_class: "local-build".to_string(),
                is_capability_binary: false,
            },
            Surface {
                path: "a.rs".to_string(),
                line: 5,
                program: "y".to_string(),
                risk_class: "local-build".to_string(),
                is_capability_binary: false,
            },
            Surface {
                path: "a.rs".to_string(),
                line: 1,
                program: "z".to_string(),
                risk_class: "local-build".to_string(),
                is_capability_binary: false,
            },
        ];
        sort_surfaces(&mut surfaces);
        assert_eq!(surfaces[0].path, "a.rs");
        assert_eq!(surfaces[0].line, 1);
        assert_eq!(surfaces[1].line, 5);
        assert_eq!(surfaces[2].path, "b.rs");
    }

    #[test]
    fn a_test_module_is_not_a_production_surface() {
        let lines: Vec<&str> = vec![
            "fn production() {",
            "    let _ = Command::new(\"git\");",
            "}",
            "#[cfg(test)]",
            "mod tests {",
            "    fn helper() { let _ = Command::new(\"git\"); }",
            "}",
        ];
        assert!(!is_test_surface("crates/x/src/a.rs", &lines, 1));
        assert!(is_test_surface("crates/x/src/a.rs", &lines, 5));
    }

    #[test]
    fn an_integration_test_file_is_not_a_production_surface() {
        let lines: Vec<&str> = vec!["fn helper() { let _ = Command::new(\"git\"); }"];
        assert!(is_test_surface("crates/x/tests/a.rs", &lines, 0));
    }

    #[test]
    fn relative_path_to_workspace() {
        let root = Path::new("/tmp/ws");
        let path = Path::new("/tmp/ws/crates/foo.rs");
        assert_eq!(relative_to_workspace(root, path), "crates/foo.rs");
        let outside = Path::new("/elsewhere/bar.rs");
        assert_eq!(relative_to_workspace(root, outside), "/elsewhere/bar.rs");
    }
}
