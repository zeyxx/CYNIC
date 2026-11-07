//! Détection des dérives du glossaire normatif (Cycle 3.3, phase Détecter).
//!
//! Le glossaire `contracts/glossary.json` est la source de vérité du vocabulaire
//! canonique CYNIC. Les synonymes bannis sont dérivés dynamiquement de ce
//! fichier à l'exécution : aucun terme banni n'est codé en dur ici, de sorte
//! que toute évolution du glossaire est reflétée sans toucher à ce test.
//!
//! Ce test échoue tant que du code ou de la documentation encore en usage
//! emploie un terme banni ; il deviendra vert lorsque les définitions d'origine
//! auront été renommées vers leurs formes canoniques (phases suivantes).
//!
//! Exemptions explicites et bornées (seules exemptions autorisées) :
//! - le glossaire lui-même (`contracts/glossary.json`) : il liste les termes
//!   bannis par définition ;
//! - les fixtures passives de tests (données non exécutables) : elles sont des
//!   traces de vérification, pas du vocabulaire actif ;
//! - le fichier d'alias de compatibilité historique (`deployment_tier.rs` dans
//!   `cynic-protocol`) : ses alias de compatibilité sont documentés comme
//!   transitoires et ne constituent pas un usage canonique ;
//! - ce test lui-même : il manipule les règles, pas le vocabulaire métier.
//!
//! Tout autre fichier sous `crates/` ou `docs/` est scanné sans exception.

#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

fn glossary_path(root: &Path) -> PathBuf {
    root.join("contracts").join("glossary.json")
}

/// Lit le glossaire et renvoie une table `terme banni -> terme canonique`,
/// dérivée de chaque entrée `banned_synonyms` des termes du glossaire.
fn banned_to_canonical(g: &serde_json::Value) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Some(terms) = g.get("terms").and_then(|t| t.as_object()) {
        for (canonical, entry) in terms {
            if let Some(synonyms) = entry
                .get("banned_synonyms")
                .and_then(|s| s.as_array())
            {
                for syn in synonyms {
                    if let Some(s) = syn.as_str() {
                        map.insert(s.to_string(), canonical.clone());
                    }
                }
            }
        }
    }
    map
}

fn load_glossary() -> serde_json::Value {
    let text = std::fs::read_to_string(glossary_path(&workspace_root()))
        .expect("read contracts/glossary.json");
    serde_json::from_str(&text).expect("parse contracts/glossary.json")
}

fn read_glossary() -> (serde_json::Value, BTreeMap<String, String>) {
    let g = load_glossary();
    let map = banned_to_canonical(&g);
    (g, map)
}

fn normalized(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Vrai si le chemin est couvert par une exemption explicite et bornée.
fn is_exempt(rel: &str) -> bool {
    // Le glossaire lui-même : il énumère les termes bannis.
    if rel == "contracts/glossary.json" {
        return true;
    }
    // Ce test : il manipule les règles de détection, pas le vocabulaire métier.
    if rel == "crates/cynic-audit-engine/tests/glossary_drift.rs" {
        return true;
    }
    // Alias de compatibilité historique : fichier unique, alias transitoires.
    if rel == "crates/cynic-protocol/src/deployment_tier.rs" {
        return true;
    }
    // Fixtures passives : données non exécutables, traces de vérification.
    if rel.contains("/tests/fixtures/")
        || rel.contains("/tests/parity/")
        || rel.contains("/tests/snapshots/")
        || rel.starts_with("crates/")
            && rel.contains("/tests/")
            && (rel.ends_with(".json")
                || rel.ends_with(".jsonl")
                || rel.ends_with(".txt")
                || rel.ends_with(".snap"))
    {
        return true;
    }
    false
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs" || e == "md") {
            out.push(p);
        }
    }
}

/// Garde-fou : les deux termes canoniques frappés par les dérives historiques
/// doivent exister dans le glossaire avec une liste `banned_synonyms` non vide.
/// Sans cette garde, un glossaire vidé de ses synonymes rendrait le scan
/// ci-dessous vacuously vert.
#[test]
fn glossary_guard_canon_terms_have_banned_synonyms() {
    let (g, _map) = read_glossary();
    let terms = g
        .get("terms")
        .and_then(|t| t.as_object())
        .expect("glossary terms object");
    for canon in ["DeploymentTier", "RefactorCollisionRisk"] {
        let entry = terms
            .get(canon)
            .unwrap_or_else(|| panic!("glossary must contain canonical term {canon}"));
        let syns = entry
            .get("banned_synonyms")
            .and_then(|s| s.as_array())
            .unwrap_or_else(|| panic!("{canon} must carry banned_synonyms"));
        assert!(
            !syns.is_empty(),
            "{canon} must declare non-empty banned_synonyms (anti-vacuity guard)"
        );
    }
}

/// Scan de `crates/` et `docs/` (`*.rs`, `*.md`) : aucun terme banni dérivé
/// du glossaire ne doit subsister hors exemptions bornées.
/// Échoue aujourd'hui sur les dérives en cours ; deviendra vert après les
/// renommages des définitions d'origine vers les formes canoniques.
#[test]
fn glossary_drift_no_banned_synonyms_in_code_or_docs() {
    let root = workspace_root();
    let (_g, banned) = read_glossary();

    // Anti-vacuité : une liste vide ne doit jamais valider le scan.
    assert!(
        !banned.is_empty(),
        "banned synonym list derived from glossary is empty; refusing vacuous pass"
    );

    let mut files = Vec::new();
    collect_files(&root.join("crates"), &mut files);
    collect_files(&root.join("docs"), &mut files);
    files.sort();
    assert!(!files.is_empty(), "no source files discovered to scan");

    let mut hits: Vec<String> = Vec::new();
    for file in &files {
        let rel = normalized(&root, file);
        if is_exempt(&rel) {
            continue;
        }
        let text = match std::fs::read_to_string(file) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for (idx, line) in text.lines().enumerate() {
            for (banned_term, canonical) in &banned {
                if line.contains(banned_term.as_str()) {
                    hits.push(format!(
                        "{}:{}: '{}' -> utilisez '{}'",
                        rel,
                        idx + 1,
                        banned_term,
                        canonical
                    ));
                    break;
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "glossary drift: {} banned synonym occurrence(s) outside bounded exemptions:\n{}",
        hits.len(),
        hits.join("\n")
    );
}
