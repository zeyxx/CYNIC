use crate::domain::ast::CorpusManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Sémantique des arêtes du graphe de connaissances.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeEdgeKind {
    /// Lien hypertexte explicite dans la prose Markdown `[label](target.md)`.
    CitesDocument,
    /// Lien contractuel formel (contrat normatif -> test de validation / port).
    EnforcesContract,
    /// Inclusion structurelle hiérarchique (Document -> Section -> Sous-section).
    ParentSection,
    /// Dépendance topologique d'une crate Rust vers une autre.
    CrateDependency,
}

/// Une arête orientée dans le graphe de connaissances CYNIC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub source: String,
    pub target: String,
    pub kind: KnowledgeEdgeKind,
    pub label: Option<String>,
}

/// Graphe de relations structurelles et causales de la base de connaissances.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    /// Nœuds déclarés (chemins relatifs canoniques).
    pub nodes: HashSet<String>,
    /// Arêtes orientées reliant les nœuds.
    pub edges: Vec<KnowledgeEdge>,
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

impl KnowledgeGraph {
    /// Construit le graphe à partir d'un CorpusManifest et de la racine du repository.
    pub fn build_from_manifest<P: AsRef<Path>>(repo_root: P, manifest: &CorpusManifest) -> Self {
        let root = repo_root.as_ref();
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

                let resolved_rel = if root.join(&cand1).exists() {
                    Some(normalize_path(&cand1))
                } else if root.join(&cand2).exists() {
                    Some(normalize_path(&cand2))
                } else {
                    None
                };

                match resolved_rel {
                    Some(target_clean) => {
                        nodes.insert(target_clean.clone());
                        edges.push(KnowledgeEdge {
                            source: doc.source_path.clone(),
                            target: target_clean,
                            kind: KnowledgeEdgeKind::CitesDocument,
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
        extract_prose_contract_references(root, &manifest.documents, &mut edges, &mut nodes);

        // 4. Lier les contrats normatifs aux tests de gouvernance connus
        wire_contract_enforcements(&mut edges, &mut nodes);

        // 5. Dépendances topologiques de crates (infra/crate-topology.json)
        wire_crate_topology(root, &mut edges, &mut nodes);

        Self {
            nodes,
            edges,
            broken_references,
        }
    }

    /// Liste des arêtes sortantes d'un document.
    pub fn outgoing_edges(&self, source: &str) -> Vec<&KnowledgeEdge> {
        self.edges.iter().filter(|e| e.source == source).collect()
    }

    /// Liste des arêtes entrantes vers un document.
    pub fn incoming_edges(&self, target: &str) -> Vec<&KnowledgeEdge> {
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

fn wire_contract_enforcements(edges: &mut Vec<KnowledgeEdge>, nodes: &mut HashSet<String>) {
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
        edges.push(KnowledgeEdge {
            source: contract.to_string(),
            target: test_file.to_string(),
            kind: KnowledgeEdgeKind::EnforcesContract,
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
    root: &Path,
    documents: &[crate::domain::ast::DocumentTree],
    edges: &mut Vec<KnowledgeEdge>,
    nodes: &mut HashSet<String>,
) {
    for doc in documents {
        let mut text_buf = String::new();
        collect_text_recursive(&doc.root_sections, &mut text_buf);
        for word in text_buf.split(|c: char| c.is_whitespace() || c == '`' || c == '(' || c == ')' || c == '[' || c == ']' || c == '"' || c == '\'') {
            if word.starts_with("contracts/") && word.ends_with(".json") {
                let clean = word.trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == ':');
                if root.join(clean).is_file() && doc.source_path != clean {
                    nodes.insert(clean.to_string());
                    if !edges.iter().any(|e| e.source == doc.source_path && e.target == clean) {
                        edges.push(KnowledgeEdge {
                            source: doc.source_path.clone(),
                            target: clean.to_string(),
                            kind: KnowledgeEdgeKind::EnforcesContract,
                            label: Some("governed_by".to_string()),
                        });
                    }
                }
            }
        }
    }
}

fn wire_crate_topology(root: &Path, edges: &mut Vec<KnowledgeEdge>, nodes: &mut HashSet<String>) {
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
                                            edges.push(KnowledgeEdge {
                                                source: crate_path.clone(),
                                                target: dep_path,
                                                kind: KnowledgeEdgeKind::CrateDependency,
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
        assert_eq!(edges[0].kind, KnowledgeEdgeKind::CrateDependency);
        assert!(nodes.contains("crates/crate-a"));
        assert!(nodes.contains("crates/crate-b"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
