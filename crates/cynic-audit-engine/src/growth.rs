//! Growth report: measured numbers per crate, grouped by layer.
//!
//! WHY. The workspace grew from 333 k to 411 k Rust lines between 2026-09-28
//! and 2026-10-26 while cleanup commits kept landing. Nothing measures growth
//! per layer, so nothing can say where it comes from or whether a cleanup held.
//! This report is the baseline. It judges nothing and blocks nothing: a number
//! becomes a shrink-only gate only after its starting value has been observed.
//!
//! GROUPING. Layers come from `infra/crate-topology.json` (`layers`). The same
//! number means different things at different layers: a line added to L0 is
//! read by every crate above it, a line added to L4 is read by none.
//!
//! The numbers, per crate:
//!   - `src_lines`, `test_lines`   size, split so test growth is visible
//!   - `files_over_1000`           files that no reader holds in mind at once
//!   - `fns_over_100`              functions longer than 100 lines (approximate)
//!   - `dead_code_allows`          lints silenced instead of code removed
//!   - `declared_inversions`       module inversions the ARCH-005 ledger declares
//!   - `duplicate_type_names`      `pub` type names defined in 2+ crates
//!
//! `declared_inversions` is READ from `infra/module-inversion-ledger.json`, not
//! recomputed: ARCH-005 owns that measurement and a second counter would drift.
//!
//! The function-length and type-name scans are textual, not parsed. Braces
//! inside string and char literals are skipped, and everything after the first
//! top-level `#[cfg(test)]` is ignored, which is the convention in this tree.
//! The numbers are a baseline for comparison with themselves, not a verdict.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::scanner::WorkspaceSourceIndex;

pub const GROWTH_SCHEMA: &str = "cynic.audit_growth.v1";

const TOPOLOGY_SSOT: &str = "infra/crate-topology.json";
const LEDGER_SSOT: &str = "infra/module-inversion-ledger.json";
const LONG_FILE_LINES: usize = 1000;
const LONG_FN_LINES: usize = 100;
const UNLAYERED: &str = "unlayered";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CrateGrowth {
    pub name: String,
    pub src_lines: usize,
    pub test_lines: usize,
    pub files_over_1000: usize,
    pub fns_over_100: usize,
    pub dead_code_allows: usize,
    pub declared_inversions: usize,
    pub duplicate_type_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LayerGrowth {
    pub layer: String,
    pub src_lines: usize,
    pub test_lines: usize,
    pub crates: Vec<CrateGrowth>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GrowthReport {
    pub schema: String,
    pub layers: Vec<LayerGrowth>,
}

/// `crates/<name>/<rest>` split into its two parts.
fn crate_and_rest(relative_path: &str) -> Option<(&str, &str)> {
    relative_path.strip_prefix("crates/")?.split_once('/')
}

/// Layer of each crate, as declared by the topology SSOT.
fn declared_layers(index: &WorkspaceSourceIndex) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(file) = index.json_files.iter().find(|f| f.relative_path == TOPOLOGY_SSOT) else {
        return out;
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&file.content) else {
        return out;
    };
    let Some(layers) = doc.get("layers").and_then(|v| v.as_object()) else {
        return out;
    };
    for (layer, members) in layers {
        for member in members.as_array().into_iter().flatten() {
            if let Some(name) = member.as_str() {
                out.insert(name.to_string(), layer.clone());
            }
        }
    }
    out
}

/// Declared module inversions, summed per crate from the ARCH-005 ledger.
fn declared_inversions(index: &WorkspaceSourceIndex) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let Some(file) = index.json_files.iter().find(|f| f.relative_path == LEDGER_SSOT) else {
        return out;
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&file.content) else {
        return out;
    };
    for entry in doc.get("entries").and_then(|v| v.as_array()).into_iter().flatten() {
        let identity = entry.get("identity").and_then(|v| v.as_str());
        let count = entry.get("count").and_then(|v| v.as_u64());
        if let (Some(identity), Some(count)) = (identity, count)
            && let Some((krate, _)) = identity.split_once('|')
        {
            *out.entry(krate.to_string()).or_insert(0) += count as usize;
        }
    }
    out
}

/// Source with string literals, char literals and `//` comments blanked, so a
/// brace inside any of them cannot move the depth counter.
fn code_only(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut in_str = false;
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k];
        if in_str {
            if c == '\\' {
                k += 1;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '/' && chars.get(k + 1) == Some(&'/') {
            break;
        } else if c == '\'' && chars.get(k + 2) == Some(&'\'') {
            k += 2;
        } else {
            out.push(c);
        }
        k += 1;
    }
    out
}

/// Lines before the first top-level `#[cfg(test)]`.
fn production_lines(content: &str) -> Vec<&str> {
    content
        .lines()
        .take_while(|l| !l.starts_with("#[cfg(test)]"))
        .collect()
}

fn is_fn_declaration(trimmed: &str) -> bool {
    let mut rest = trimmed;
    if let Some(after) = rest.strip_prefix("pub") {
        rest = after.trim_start();
        if rest.starts_with('(') {
            rest = rest.split_once(')').map_or("", |(_, tail)| tail).trim_start();
        }
    }
    for modifier in ["const ", "async ", "unsafe "] {
        rest = rest.strip_prefix(modifier).unwrap_or(rest);
    }
    rest.starts_with("fn ")
}

/// Number of functions whose body spans more than `limit` lines.
fn long_fn_count(lines: &[&str], limit: usize) -> usize {
    let mut count = 0;
    for (start, line) in lines.iter().enumerate() {
        if !is_fn_declaration(line.trim_start()) {
            continue;
        }
        let mut depth: i64 = 0;
        let mut opened = false;
        for (offset, body) in lines[start..].iter().enumerate() {
            let code = code_only(body);
            if !opened && code.trim_end().ends_with(';') {
                break; // declaration without a body (trait method)
            }
            for c in code.chars() {
                match c {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if opened && depth <= 0 {
                if offset + 1 > limit {
                    count += 1;
                }
                break;
            }
        }
    }
    count
}

/// Name of a `pub struct|enum|trait` declared on this line, if any.
fn public_type_name(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("pub ")?;
    let rest = ["struct ", "enum ", "trait "]
        .iter()
        .find_map(|kw| rest.strip_prefix(kw))?;
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    (end > 0).then(|| &rest[..end])
}

#[derive(Default)]
struct Tally {
    src_lines: usize,
    test_lines: usize,
    files_over_1000: usize,
    fns_over_100: usize,
    dead_code_allows: usize,
}

pub fn growth_report(index: &WorkspaceSourceIndex) -> GrowthReport {
    let layers = declared_layers(index);
    let inversions = declared_inversions(index);

    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    let mut type_homes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for file in &index.rust_files {
        let Some((krate, rest)) = crate_and_rest(&file.relative_path) else {
            continue;
        };
        let tally = tallies.entry(krate.to_string()).or_default();
        if rest.starts_with("tests/") || rest.starts_with("benches/") {
            tally.test_lines += file.line_count;
            continue;
        }
        if !rest.starts_with("src/") {
            continue;
        }
        tally.src_lines += file.line_count;
        if file.line_count > LONG_FILE_LINES {
            tally.files_over_1000 += 1;
        }
        let lines = production_lines(&file.content);
        tally.fns_over_100 += long_fn_count(&lines, LONG_FN_LINES);
        tally.dead_code_allows += lines.iter().filter(|l| l.contains("allow(dead_code)")).count();
        for line in &lines {
            if let Some(name) = public_type_name(line.trim_start()) {
                type_homes.entry(name.to_string()).or_default().insert(krate.to_string());
            }
        }
    }

    let mut duplicates: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, homes) in &type_homes {
        if homes.len() > 1 {
            for home in homes {
                duplicates.entry(home.clone()).or_default().push(name.clone());
            }
        }
    }

    let mut by_layer: BTreeMap<String, Vec<CrateGrowth>> = BTreeMap::new();
    for (name, tally) in tallies {
        let layer = layers.get(&name).cloned().unwrap_or_else(|| UNLAYERED.to_string());
        by_layer.entry(layer).or_default().push(CrateGrowth {
            declared_inversions: inversions.get(&name).copied().unwrap_or(0),
            duplicate_type_names: duplicates.remove(&name).unwrap_or_default(),
            src_lines: tally.src_lines,
            test_lines: tally.test_lines,
            files_over_1000: tally.files_over_1000,
            fns_over_100: tally.fns_over_100,
            dead_code_allows: tally.dead_code_allows,
            name,
        });
    }

    let mut layers_out: Vec<LayerGrowth> = by_layer
        .into_iter()
        .map(|(layer, crates)| LayerGrowth {
            layer,
            src_lines: crates.iter().map(|c| c.src_lines).sum(),
            test_lines: crates.iter().map(|c| c.test_lines).sum(),
            crates,
        })
        .collect();
    // L0..L4 first, then cross_cutting, then unlayered: alphabetical already
    // gives L0..L4 and cross_cutting; only `unlayered` must be moved last.
    layers_out.sort_by(|a, b| {
        (a.layer == UNLAYERED, &a.layer).cmp(&(b.layer == UNLAYERED, &b.layer))
    });

    GrowthReport {
        schema: GROWTH_SCHEMA.to_string(),
        layers: layers_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::SourceFile;
    use std::path::PathBuf;

    fn file(path: &str, content: &str) -> SourceFile {
        SourceFile {
            relative_path: path.to_string(),
            absolute_path: PathBuf::from(path),
            line_count: content.lines().count(),
            content: content.to_string(),
        }
    }

    fn long_fn_source(body_lines: usize) -> String {
        let mut s = String::from("pub fn big() {\n");
        for _ in 0..body_lines {
            s.push_str("    let _x = 1;\n");
        }
        s.push_str("}\n");
        s
    }

    #[test]
    fn a_brace_inside_a_string_does_not_move_the_depth() {
        let src = "fn f() {\n    let s = \"}}}\";\n    let c = '{';\n}\n";
        let lines: Vec<&str> = src.lines().collect();
        assert_eq!(long_fn_count(&lines, 3), 1, "4 lines > limit 3, one fn");
        assert_eq!(long_fn_count(&lines, 4), 0, "4 lines is not > 4");
    }

    #[test]
    fn a_trait_method_without_body_is_not_a_function_body() {
        let src = "pub trait T {\n    fn a(&self);\n    fn b(&self);\n}\n";
        let lines: Vec<&str> = src.lines().collect();
        assert_eq!(long_fn_count(&lines, 0), 0);
    }

    #[test]
    fn everything_after_cfg_test_is_ignored() {
        let src = format!("fn ok() {{}}\n#[cfg(test)]\nmod tests {{\n{}}}\n", long_fn_source(200));
        assert_eq!(long_fn_count(&production_lines(&src), 100), 0);
    }

    #[test]
    fn report_groups_by_declared_layer_and_unlayered_goes_last() {
        let topology = r#"{"layers":{"L0":["cynic-a"],"L1":["cynic-b"]}}"#;
        let index = WorkspaceSourceIndex {
            rust_files: vec![
                file("crates/cynic-a/src/lib.rs", "pub struct Claim;\n"),
                file("crates/cynic-b/src/lib.rs", "pub struct Claim;\n#[allow(dead_code)]\nfn x() {}\n"),
                file("crates/cynic-c/src/lib.rs", "pub enum Solo {}\n"),
                file("crates/cynic-a/tests/t.rs", "fn t() {}\nfn u() {}\n"),
            ],
            json_files: vec![file(TOPOLOGY_SSOT, topology)],
            ..WorkspaceSourceIndex::default()
        };
        let report = growth_report(&index);
        let names: Vec<&str> = report.layers.iter().map(|l| l.layer.as_str()).collect();
        assert_eq!(names, ["L0", "L1", "unlayered"]);

        let a = &report.layers[0].crates[0];
        assert_eq!(a.test_lines, 2);
        assert_eq!(a.duplicate_type_names, ["Claim"]);
        let b = &report.layers[1].crates[0];
        assert_eq!(b.dead_code_allows, 1);
        assert_eq!(b.duplicate_type_names, ["Claim"]);
        let c = &report.layers[2].crates[0];
        assert!(c.duplicate_type_names.is_empty());
    }

    #[test]
    fn inversions_are_read_from_the_ledger_per_crate() {
        let ledger = r#"{"entries":[
            {"identity":"cynic-host|domain -> execution","count":4},
            {"identity":"cynic-host|domain -> x","count":1},
            {"identity":"cynic-node|usecases -> y","count":2}]}"#;
        let index = WorkspaceSourceIndex {
            json_files: vec![file(LEDGER_SSOT, ledger)],
            ..WorkspaceSourceIndex::default()
        };
        let counts = declared_inversions(&index);
        assert_eq!(counts["cynic-host"], 5);
        assert_eq!(counts["cynic-node"], 2);
    }

    #[test]
    fn long_files_and_long_fns_are_counted() {
        let src = long_fn_source(150);
        let index = WorkspaceSourceIndex {
            rust_files: vec![file("crates/cynic-a/src/big.rs", &src)],
            ..WorkspaceSourceIndex::default()
        };
        let c = &growth_report(&index).layers[0].crates[0];
        assert_eq!(c.fns_over_100, 1);
        assert_eq!(c.files_over_1000, 0);
    }
}
