//! Traditions — lecture depuis `tradition-data.json`. Zéro hardcoding.
//!
//! Le dataset est embarqué au compile-time via `include_str!`. Le code ne contient
//! aucune donnée statique — tout est externalisé dans `tradition-data.json`.

use serde::Deserialize;
use std::collections::HashMap;

/// Chemin relatif au crate du fichier JSON embarqué.
const TRADITION_DATA_JSON: &str = include_str!("../../docs/v2-architecture-canon/tradition-data.json");

/// Structure d'une tradition dans le JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct TraditionData {
    pub id: String,
    pub name: String,
    pub epoch: String,
    pub mechanism: String,
    pub scale: String,
    pub coordinates: Coordinates,
    pub core: String,
    pub institutions: Vec<String>,
    pub coordination: String,
    #[serde(default)]
    pub population_estimate: Option<u64>,
    #[serde(default)]
    pub duration_years: Option<u64>,
}

/// Coordonnées (s,b,n) dans l'espace de phase.
#[derive(Debug, Clone, Deserialize)]
pub struct Coordinates {
    pub s: f64,
    pub b: f64,
    pub n: f64,
}

/// Dataset complet.
#[derive(Debug, Clone, Deserialize)]
pub struct TraditionDataset {
    pub version: String,
    pub last_updated: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub traditions: Vec<TraditionData>,
}

/// Charge le dataset depuis le JSON embarqué.
///
/// # Erreur
/// Retourne une erreur de parsing si le JSON est invalide.
pub fn load_tradition_dataset() -> Result<TraditionDataset, serde_json::Error> {
    serde_json::from_str(TRADITION_DATA_JSON)
}

/// Charge les traditions comme hashmap indexée par ID.
pub fn traditions_by_id() -> Result<HashMap<String, TraditionData>, serde_json::Error> {
    let dataset = load_tradition_dataset()?;
    let mut map = HashMap::new();
    for t in dataset.traditions {
        map.insert(t.id.clone(), t);
    }
    Ok(map)
}

/// Retourne le nombre de traditions dans le dataset.
pub fn tradition_count() -> Result<usize, serde_json::Error> {
    Ok(load_tradition_dataset()?.traditions.len())
}

/// Calcule G depuis les coordonnées d'une tradition.
/// G = min(s/0.33, b/0.33, n/0.34)
pub fn g_from_tradition(t: &TraditionData) -> f64 {
    (t.coordinates.s / 0.33)
        .min(t.coordinates.b / 0.33)
        .min(t.coordinates.n / 0.34)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_loads() {
        let dataset = load_tradition_dataset().expect("Dataset doit parser");
        assert!(!dataset.traditions.is_empty());
    }

    #[test]
    fn test_version_present() {
        let dataset = load_tradition_dataset().expect("Dataset doit parser");
        assert!(!dataset.version.is_empty());
    }

    #[test]
    fn test_tradition_count_positive() {
        let count = tradition_count().expect("Doit compter");
        assert!(count > 0);
    }

    #[test]
    fn test_g_calculation() {
        let dataset = load_tradition_dataset().expect("Dataset doit parser");
        for t in &dataset.traditions {
            let g = g_from_tradition(t);
            assert!(g > 0.0, "G doit être positif pour {}", t.id);
        }
    }

    #[test]
    fn test_coordinates_sum_to_one() {
        let dataset = load_tradition_dataset().expect("Dataset doit parser");
        for t in &dataset.traditions {
            let sum = t.coordinates.s + t.coordinates.b + t.coordinates.n;
            assert!(
                (sum - 1.0).abs() < 0.02,
                "Coordonnées de {} devraient sommer ≈ 1, got {}",
                t.id,
                sum
            );
        }
    }

    #[test]
    fn test_all_ids_unique() {
        let dataset = load_tradition_dataset().expect("Dataset doit parser");
        let mut ids: Vec<&str> = dataset.traditions.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(
            unique_count,
            ids.len(),
            "Tous les IDs doivent être uniques"
        );
    }
}
