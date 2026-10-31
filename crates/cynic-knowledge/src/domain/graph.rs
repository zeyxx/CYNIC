use crate::domain::ast::CorpusManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Sémantique des arêtes du graphe de connaissances.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CorpusEdgeKind {
    /// Lien hypertexte explicite dans la prose Markdown `[label](target.md)`.
    CitesDocument,
    /// Lien contractuel formel (contrat normatif -> test de validation / port).
    EnforcesContract,
    /// Inclusion structurelle hiérarchique (Document -> Section -> Sous-section).
    ParentSection,
    /// Dépendance topologique d'une crate Rust vers une autre.
    CrateDependency,
    /// Import de module Rust (use crate::... ou use super::...).
    UsesModule,
    /// Implémentation de trait Rust (impl Trait for Type).
    ImplementsTrait,
    /// Déclaration de module Rust (mod foo;).
    DeclaresModule,
    /// Implémentation inhérente Rust (impl Type).
    InherentImpl,
}

/// Une arête orientée dans le graphe de connaissances CYNIC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusEdge {
    pub source: String,
    pub target: String,
    pub kind: CorpusEdgeKind,
    pub label: Option<String>,
}

/// Graphe de relations structurelles et causales de la base de connaissances.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorpusGraph {
    /// Nœuds déclarés (chemins relatifs canoniques).
    pub nodes: HashSet<String>,
    /// Arêtes orientées reliant les nœuds.
    pub edges: Vec<CorpusEdge>,
    /// Références cassées détectées lors de la résolution.
    pub broken_references: Vec<BrokenReference>,
}

/// Référence brisée pointant vers un fichier inexistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokenReference {
    pub source_doc: String,
    pub target_raw: String,
    pub resolved_attempt: String,
}

impl CorpusGraph {
    /// Construit le graphe à partir d'un CorpusManifest et de la racine du repository.
    /// Adapter I/O: collecte les chemins connus du workspace puis délègue au cœur pur.
    pub fn build_from_manifest<P: AsRef<Path>>(repo_root: P, manifest: &CorpusManifest) -> Self {
        let root = repo_root.as_ref();
        // Collecte I/O des chemins connus (adapter).
        let mut known_paths: Vec<PathBuf> = Vec::new();
        if let Ok(entries) = collect_workspace_paths(root) {
            known_paths = entries;
        }
        let mut graph = Self::build_from_manifest_with_paths(known_paths, manifest);
        // Câblage topologique des crates (I/O adapter).
        wire_crate_topology(root, &mut graph.edges, &mut graph.nodes);
        // Câblage des imports Rust (use crate::..., use super::...).
        wire_rust_use_edges(root, &mut graph.edges, &mut graph.nodes);
        // Câblage des impl Traits Rust (impl Trait for Type).
        wire_rust_impl_traits(root, &mut graph.edges, &mut graph.nodes);
        graph
    }

    /// Variante pure (Zero-I/O): construit le graphe à partir d'un ensemble en mémoire
    /// de chemins connus du workspace.
    pub fn build_from_manifest_with_paths<I, P>(
        known_paths: I,
        manifest: &CorpusManifest,
    ) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let known: HashSet<PathBuf> = known_paths.into_iter().map(Into::into).collect();
        let mut nodes = HashSet::new();
        let mut edges = Vec::new();
        let mut broken_references = Vec::new();

        // 1. Enregistrer tous les documents comme nœuds
        for doc in &manifest.documents {
            nodes.insert(doc.source_path.clone());
        }

        // 2. Extraire et résoudre les liens Markdown
        for doc in &manifest.documents {
            let doc_dir = Path::new(&doc.source_path)
                .parent()
                .unwrap_or_else(|| Path::new(""));

            let mut links_to_process = Vec::new();
            collect_links_recursive(&doc.root_sections, &mut links_to_process);

            for (link, _sec_title) in links_to_process {
                let raw_target = link.target.split('#').next().unwrap_or("").trim();

                // Ignorer les ancres pures dans la même page (#ancre) ou liens web
                if raw_target.is_empty()
                    || raw_target.starts_with("http://")
                    || raw_target.starts_with("https://")
                    || raw_target.starts_with("mailto:")
                {
                    continue;
                }

                // Résolution par rapport au dossier du doc ou à la racine
                let target_path_buf = PathBuf::from(raw_target);
                let cand1 = doc_dir.join(&target_path_buf);
                let cand2 = target_path_buf;

                let resolved_rel = if path_known(&known, &cand1) {
                    Some(normalize_path(&cand1))
                } else if path_known(&known, &cand2) {
                    Some(normalize_path(&cand2))
                } else {
                    None
                };

                match resolved_rel {
                    Some(target_clean) => {
                        nodes.insert(target_clean.clone());
                        edges.push(CorpusEdge {
                            source: doc.source_path.clone(),
                            target: target_clean,
                            kind: CorpusEdgeKind::CitesDocument,
                            label: Some(link.label.clone()),
                        });
                    }
                    None => {
                        broken_references.push(BrokenReference {
                            source_doc: doc.source_path.clone(),
                            target_raw: raw_target.to_string(),
                            resolved_attempt: cand1.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }

        // 3. Extraire les références contractuelles implicites dans la prose Markdown
        extract_prose_contract_references(&known, &manifest.documents, &mut edges, &mut nodes);

        // 4. Lier les contrats normatifs aux tests de gouvernance connus
        wire_contract_enforcements(&mut edges, &mut nodes);

        Self {
            nodes,
            edges,
            broken_references,
        }
    }

    /// Liste des arêtes sortantes d'un document.
    pub fn outgoing_edges(&self, source: &str) -> Vec<&CorpusEdge> {
        self.edges.iter().filter(|e| e.source == source).collect()
    }

    /// Liste des arêtes entrantes vers un document.
    pub fn incoming_edges(&self, target: &str) -> Vec<&CorpusEdge> {
        self.edges.iter().filter(|e| e.target == target).collect()
    }

    /// Documents orphelins (qui ne sont cités par aucun autre document).
    pub fn orphan_documents(&self) -> Vec<String> {
        let cited_targets: HashSet<&str> = self.edges.iter().map(|e| e.target.as_str()).collect();
        let mut orphans: Vec<String> = self
            .nodes
            .iter()
            .filter(|n| n.ends_with(".md") && !cited_targets.contains(n.as_str()))
            .cloned()
            .collect();
        orphans.sort();
        orphans
    }
}

fn collect_links_recursive<'a>(
    sections: &'a [crate::domain::ast::SectionNode],
    acc: &mut Vec<(&'a crate::domain::ast::MarkdownLink, &'a str)>,
) {
    for sec in sections {
        for link in &sec.links {
            acc.push((link, &sec.title));
        }
        collect_links_recursive(&sec.children, acc);
    }
}

fn normalize_path(p: &Path) -> String {
    use std::path::Component;
    let mut parts = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => parts.push(s.to_string_lossy().to_string()),
            Component::ParentDir => {
                parts.pop();
            }
            Component::CurDir => {}
            _ => {}
        }
    }
    parts.join("/")
}

fn wire_contract_enforcements(edges: &mut Vec<CorpusEdge>, nodes: &mut HashSet<String>) {
    let wiring = [
        (
            "contracts/repository-governance.json",
            "crates/cynic-cli/tests/repository_governance.rs",
        ),
        (
            "contracts/publication.json",
            "crates/cynic-cli/tests/publication_contract.rs",
        ),
        (
            "contracts/foundation.json",
            "crates/cynic-kernel/tests/architecture_boundary.rs",
        ),
    ];

    for (contract, test_file) in wiring {
        nodes.insert(contract.to_string());
        nodes.insert(test_file.to_string());
        edges.push(CorpusEdge {
            source: contract.to_string(),
            target: test_file.to_string(),
            kind: CorpusEdgeKind::EnforcesContract,
            label: Some("verified_by".to_string()),
        });
    }
}

fn collect_text_recursive(
    sections: &[crate::domain::ast::SectionNode],
    acc: &mut String,
) {
    for sec in sections {
        acc.push_str(&sec.title);
        acc.push(' ');
        acc.push_str(&sec.content);
        acc.push(' ');
        collect_text_recursive(&sec.children, acc);
    }
}

fn extract_prose_contract_references(
    known: &HashSet<PathBuf>,
    documents: &[crate::domain::ast::DocumentTree],
    edges: &mut Vec<CorpusEdge>,
    nodes: &mut HashSet<String>,
) {
    for doc in documents {
        let mut text_buf = String::new();
        collect_text_recursive(&doc.root_sections, &mut text_buf);
        for word in text_buf.split(|c: char| c.is_whitespace() || c == '`' || c == '(' || c == ')' || c == '[' || c == ']' || c == '"' || c == '\'') {
            if word.starts_with("contracts/") && word.ends_with(".json") {
                let clean = word.trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == ':');
                if path_known(known, Path::new(clean)) && doc.source_path != clean {
                    nodes.insert(clean.to_string());
                    if !edges.iter().any(|e| e.source == doc.source_path && e.target == clean) {
                        edges.push(CorpusEdge {
                            source: doc.source_path.clone(),
                            target: clean.to_string(),
                            kind: CorpusEdgeKind::EnforcesContract,
                            label: Some("governed_by".to_string()),
                        });
                    }
                }
            }
        }
    }
}

fn wire_rust_use_edges(root: &Path, edges: &mut Vec<CorpusEdge>, nodes: &mut HashSet<String>) {
    use std::fs;
    use walkdir::WalkDir;

    let crates_dir = root.join("crates");
    if !crates_dir.exists() {
        return;
    }

    let mut rs_files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&crates_dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != "target" && name != ".git" && name != "node_modules"
        })
        .flatten()
    {
        if entry.file_type().is_file()
            && entry.path().extension().is_some_and(|ext| ext == "rs")
            && (entry.path().to_string_lossy().contains("/src/")
                || entry.path().to_string_lossy().contains("/tests/"))
        {
            rs_files.push(entry.path().to_path_buf());
        }
    }

    for path in &rs_files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        nodes.insert(rel_path.clone());

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("use ") {
                continue;
            }
            let module_ref = trimmed
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .split(" as ")
                .next()
                .unwrap_or("")
                .trim();

            let resolved_target = if module_ref.starts_with("crate::") {
                Some(resolve_crate_use(root, &rel_path, module_ref))
            } else if module_ref.starts_with("super::") {
                Some(resolve_super_use(root, &rel_path, module_ref))
            } else {
                None
            };

            if let Some(target) = resolved_target {
                if !target.is_empty() && target != rel_path && root.join(&target).exists() {
                    nodes.insert(target.clone());
                    if !edges
                        .iter()
                        .any(|e| e.source == rel_path && e.target == target)
                    {
                        edges.push(CorpusEdge {
                            source: rel_path.clone(),
                            target,
                            kind: CorpusEdgeKind::UsesModule,
                            label: Some("uses".to_string()),
                        });
                    }
                }
            }
        }
    }
}

fn resolve_crate_use(root: &Path, source: &str, module_ref: &str) -> String {
    let parts: Vec<&str> = module_ref.split("::").collect();
    if parts.len() < 2 {
        return String::new();
    }

    let source_parts: Vec<&str> = source.split('/').collect();
    let crate_name = if source_parts.len() >= 2 && source_parts[0] == "crates" {
        source_parts[1]
    } else {
        return String::new();
    };

    let module_path = parts[1..].join("/");
    let rs_path = format!("crates/{}/src/{}.rs", crate_name, module_path);
    if root.join(&rs_path).exists() {
        return rs_path;
    }
    let mod_path = format!("crates/{}/src/{}/mod.rs", crate_name, module_path);
    if root.join(&mod_path).exists() {
        return mod_path;
    }
    String::new()
}

fn resolve_super_use(_root: &Path, source: &str, module_ref: &str) -> String {
    let source_dir = Path::new(source).parent().unwrap_or(Path::new(""));
    let super_dir = source_dir.parent().unwrap_or(Path::new(""));
    let module_name = module_ref.trim_start_matches("super::");
    let target = super_dir.join(format!("{}.rs", module_name));
    target.to_string_lossy().to_string()
}

fn wire_rust_impl_traits(root: &Path, edges: &mut Vec<CorpusEdge>, nodes: &mut HashSet<String>) {
    use std::fs;
    use walkdir::WalkDir;

    let crates_dir = root.join("crates");
    if !crates_dir.exists() {
        return;
    }

    let mut rs_files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&crates_dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != "target" && name != ".git" && name != "node_modules"
        })
        .flatten()
    {
        if entry.file_type().is_file()
            && entry.path().extension().is_some_and(|ext| ext == "rs")
            && (entry.path().to_string_lossy().contains("/src/")
                || entry.path().to_string_lossy().contains("/tests/"))
        {
            rs_files.push(entry.path().to_path_buf());
        }
    }

    for path in &rs_files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        nodes.insert(rel_path.clone());

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Extract impl Trait for Type patterns
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("impl ") {
                continue;
            }
            // Match "impl Trait for Type" or "impl Trait for Type<T>"
            if let Some(for_idx) = trimmed.find(" for ") {
                let trait_part = trimmed[5..for_idx].trim();
                let type_part = trimmed[for_idx + 4..]
                    .trim()
                    .trim_end_matches('{')
                    .trim();
                // Clean up generics: Type<T> -> Type
                let type_clean = type_part.split('<').next().unwrap_or(type_part).trim();
                let trait_clean = trait_part.split('<').next().unwrap_or(trait_part).trim();

                if !trait_clean.is_empty() && !type_clean.is_empty() {
                    // The trait might be defined in another file, but we record the relation
                    // as a self-referential edge for now (source = file, target = file)
                    // since we don't have cross-file trait resolution yet.
                    // Instead, we record it as a node-level annotation.
                    // For now, skip cross-file resolution (requires trait registry).
                }
            }
        }
    }
}

fn wire_rust_mod_declarations(root: &Path, edges: &mut Vec<CorpusEdge>, nodes: &mut HashSet<String>) {
    use std::fs;
    use walkdir::WalkDir;

    let crates_dir = root.join("crates");
    if !crates_dir.exists() {
        return;
    }

    let mut rs_files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&crates_dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != "target" && name != ".git" && name != "node_modules"
        })
        .flatten()
    {
        if entry.file_type().is_file()
            && entry.path().extension().is_some_and(|ext| ext == "rs")
            && (entry.path().to_string_lossy().contains("/src/")
                || entry.path().to_string_lossy().contains("/tests/"))
        {
            rs_files.push(entry.path().to_path_buf());
        }
    }

    for path in &rs_files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        nodes.insert(rel_path.clone());

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("mod ") {
                continue;
            }
            let mod_name = trimmed
                .trim_start_matches("pub ")
                .trim_start_matches("mod ")
                .trim_end_matches(';')
                .trim_end_matches('{')
                .trim_end_matches('}')
                .trim();
            if mod_name.is_empty() || mod_name.contains(' ') {
                continue;
            }

            let source_dir = Path::new(&rel_path).parent().unwrap_or(Path::new(""));
            let mod_rs = source_dir.join(format!("{}.rs", mod_name));
            let mod_dir = source_dir.join(mod_name).join("mod.rs");

            let target = if root.join(&mod_rs).exists() {
                Some(mod_rs.to_string_lossy().to_string())
            } else if root.join(&mod_dir).exists() {
                Some(mod_dir.to_string_lossy().to_string())
            } else {
                None
            };

            if let Some(target_path) = target {
                if target_path != rel_path {
                    nodes.insert(target_path.clone());
                    if !edges
                        .iter()
                        .any(|e| e.source == rel_path && e.target == target_path)
                    {
                        edges.push(CorpusEdge {
                            source: rel_path.clone(),
                            target: target_path,
                            kind: CorpusEdgeKind::DeclaresModule,
                            label: Some("declares".to_string()),
                        });
                    }
                }
            }
        }
    }
}

fn wire_rust_inherent_impls(root: &Path, edges: &mut Vec<CorpusEdge>, nodes: &mut HashSet<String>) {
    use std::fs;
    use walkdir::WalkDir;

    let crates_dir = root.join("crates");
    if !crates_dir.exists() {
        return;
    }

    let mut rs_files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&crates_dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != "target" && name != ".git" && name != "node_modules"
        })
        .flatten()
    {
        if entry.file_type().is_file()
            && entry.path().extension().is_some_and(|ext| ext == "rs")
            && (entry.path().to_string_lossy().contains("/src/")
                || entry.path().to_string_lossy().contains("/tests/"))
        {
            rs_files.push(entry.path().to_path_buf());
        }
    }

    for path in &rs_files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        nodes.insert(rel_path.clone());

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("impl ") {
                continue;
            }
            if trimmed.contains(" for ") {
                continue;
            }
            let type_part = trimmed[5..]
                .trim()
                .trim_end_matches('{')
                .trim();
            let type_clean = type_part.split('<').next().unwrap_or(type_part).trim();
            if type_clean.is_empty() || type_clean.contains(' ') {
                continue;
            }
        }
    }
}

fn wire_crate_topology(root: &Path, edges: &mut Vec<CorpusEdge>, nodes: &mut HashSet<String>) {
    let topo_path = root.join("infra/crate-topology.json");
    if let Ok(bytes) = std::fs::read(&topo_path) {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(crates) = val.get("crates").and_then(|c| c.as_object()) {
                for (crate_name, info) in crates {
                    let crate_path = format!("crates/{}", crate_name);
                    if root.join(&crate_path).exists() {
                        nodes.insert(crate_path.clone());
                        if let Some(deps) = info.get("depends_on").and_then(|d| d.as_array()) {
                            for dep in deps {
                                if let Some(dep_name) = dep.as_str() {
                                    let dep_path = format!("crates/{}", dep_name);
                                    if root.join(&dep_path).exists() {
                                        nodes.insert(dep_path.clone());
                                        if !edges.iter().any(|e| e.source == crate_path && e.target == dep_path) {
                                            edges.push(CorpusEdge {
                                                source: crate_path.clone(),
                                                target: dep_path,
                                                kind: CorpusEdgeKind::CrateDependency,
                                                label: Some("depends_on".to_string()),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


/// Backward-compatible aliases (deprecated).
#[deprecated(note = "use CorpusGraph")]
pub type KnowledgeGraph = CorpusGraph;
#[deprecated(note = "use CorpusEdge")]
pub type KnowledgeEdge = CorpusEdge;
#[deprecated(note = "use CorpusEdgeKind")]
pub type KnowledgeEdgeKind = CorpusEdgeKind;


fn collect_workspace_paths(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !["target", ".git", ".cargo", "node_modules", "shared-target"].contains(&name.as_ref())
        })
        .flatten()
    {
        if entry.file_type().is_file() {
            if let Ok(rel) = entry.path().strip_prefix(root) {
                out.push(rel.to_path_buf());
            }
        }
    }
    Ok(out)
}

fn path_known(known: &HashSet<PathBuf>, p: &Path) -> bool {
    let norm = PathBuf::from(normalize_path(p));
    if known.contains(p) || known.contains(&norm) {
        return true;
    }
    known
        .iter()
        .any(|k| k.ends_with(p) || p.ends_with(k) || k.ends_with(&norm) || norm.ends_with(k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_wire_crate_topology_from_json() {
        let temp_dir = std::env::temp_dir().join(format!("cynic_topo_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("infra")).unwrap();
        fs::create_dir_all(temp_dir.join("crates/crate-a")).unwrap();
        fs::create_dir_all(temp_dir.join("crates/crate-b")).unwrap();

        let json = r#"{
            "crates": {
                "crate-a": {
                    "depends_on": ["crate-b"]
                }
            }
        }"#;
        fs::write(temp_dir.join("infra/crate-topology.json"), json).unwrap();

        let mut edges = Vec::new();
        let mut nodes = HashSet::new();
        wire_crate_topology(&temp_dir, &mut edges, &mut nodes);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].source, "crates/crate-a");
        assert_eq!(edges[0].target, "crates/crate-b");
        assert_eq!(edges[0].kind, CorpusEdgeKind::CrateDependency);
        assert!(nodes.contains("crates/crate-a"));
        assert!(nodes.contains("crates/crate-b"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
