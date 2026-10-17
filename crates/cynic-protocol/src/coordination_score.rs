//! Coordination Score — G = min(S/S₀, B/B₀, N/N₀) — Leontief de la coordination.

use std::fmt;

/// Coordonnées (s,b,n) normalisées : s + b + n = 1
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseCoordinates {
    pub signal: f64,
    pub boucle: f64,
    pub noeud: f64,
}

impl PhaseCoordinates {
    pub fn new(signal: f64, boucle: f64, noeud: f64) -> Self {
        assert!(
            signal >= 0.0 && boucle >= 0.0 && noeud >= 0.0,
            "Coordonnées doivent être ≥ 0"
        );
        let total = signal + boucle + noeud;
        if total <= 0.0 {
            PhaseCoordinates {
                signal: 1.0 / 3.0,
                boucle: 1.0 / 3.0,
                noeud: 1.0 / 3.0,
            }
        } else {
            PhaseCoordinates {
                signal: signal / total,
                boucle: boucle / total,
                noeud: noeud / total,
            }
        }
    }

    pub fn distance(&self, other: &PhaseCoordinates) -> f64 {
        ((self.signal - other.signal).powi(2)
            + (self.boucle - other.boucle).powi(2)
            + (self.noeud - other.noeud).powi(2))
        .sqrt()
    }

    pub fn cosine_similarity(&self, other: &PhaseCoordinates) -> f64 {
        let dot =
            self.signal * other.signal + self.boucle * other.boucle + self.noeud * other.noeud;
        let n1 = (self.signal.powi(2) + self.boucle.powi(2) + self.noeud.powi(2)).sqrt();
        let n2 = (other.signal.powi(2) + other.boucle.powi(2) + other.noeud.powi(2)).sqrt();
        if n1 < f64::EPSILON || n2 < f64::EPSILON {
            0.0
        } else {
            (dot / (n1 * n2)).clamp(-1.0, 1.0)
        }
    }

    pub fn est_valide(&self) -> bool {
        (self.signal + self.boucle + self.noeud - 1.0).abs() < 1e-6
    }
}

impl fmt::Display for PhaseCoordinates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(s={:.3}, b={:.3}, n={:.3})",
            self.signal, self.boucle, self.noeud
        )
    }
}

/// Score de coordination — Leontief maillon faible
#[derive(Debug, Clone)]
pub struct CoordinationScore {
    pub signal: f64,
    pub boucle: f64,
    pub noeud: f64,
    seuil_signal: f64,
    seuil_boucle: f64,
    seuil_noeud: f64,
}

impl CoordinationScore {
    pub fn new(signal: f64, boucle: f64, noeud: f64) -> Self {
        CoordinationScore {
            signal: signal.max(0.0),
            boucle: boucle.max(0.0),
            noeud: noeud.max(0.0),
            seuil_signal: 0.1,
            seuil_boucle: 0.1,
            seuil_noeud: 0.1,
        }
    }

    pub fn with_seuils(signal: f64, boucle: f64, noeud: f64, s0: f64, b0: f64, n0: f64) -> Self {
        CoordinationScore {
            signal: signal.max(0.0),
            boucle: boucle.max(0.0),
            noeud: noeud.max(0.0),
            seuil_signal: s0.max(f64::EPSILON),
            seuil_boucle: b0.max(f64::EPSILON),
            seuil_noeud: n0.max(f64::EPSILON),
        }
    }

    pub fn g(&self) -> f64 {
        let rs = self.signal / self.seuil_signal;
        let rb = self.boucle / self.seuil_boucle;
        let rn = self.noeud / self.seuil_noeud;
        rs.min(rb).min(rn)
    }

    pub fn maillon_faible(&self) -> &'static str {
        let rs = self.signal / self.seuil_signal;
        let rb = self.boucle / self.seuil_boucle;
        let rn = self.noeud / self.seuil_noeud;
        if rs <= rb && rs <= rn {
            "signal"
        } else if rb <= rs && rb <= rn {
            "boucle"
        } else {
            "noeud"
        }
    }

    pub fn est_viable(&self) -> bool {
        self.g() >= 1.0
    }

    pub fn delta_g(&self, ds: f64, db: f64, dn: f64) -> f64 {
        let next = CoordinationScore {
            signal: (self.signal + ds).max(0.0),
            boucle: (self.boucle + db).max(0.0),
            noeud: (self.noeud + dn).max(0.0),
            ..*self
        };
        next.g() - self.g()
    }
}

/// Calcule G depuis coordonnées de phase (S₀=B₀=N₀=0.33)
pub fn g_from_coords(c: &PhaseCoordinates) -> f64 {
    (c.signal / 0.33_f64)
        .min(c.boucle / 0.33_f64)
        .min(c.noeud / 0.34_f64)
}

/// Métrique de convergence entre N traditions
#[derive(Debug, Clone)]
pub struct ConvergenceMetric {
    pub noms: Vec<String>,
    pub coords: Vec<PhaseCoordinates>,
}

impl ConvergenceMetric {
    pub fn new() -> Self {
        ConvergenceMetric {
            noms: Vec::new(),
            coords: Vec::new(),
        }
    }

    pub fn ajouter(&mut self, nom: &str, s: f64, b: f64, n: f64) {
        self.noms.push(nom.to_string());
        self.coords.push(PhaseCoordinates::new(s, b, n));
    }

    pub fn len(&self) -> usize {
        self.coords.len()
    }

    pub fn convergence(&self) -> f64 {
        let n = self.coords.len();
        if n < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        let mut count = 0u64;
        for i in 0..n {
            for j in (i + 1)..n {
                total += self.coords[i].distance(&self.coords[j]);
                count += 1;
            }
        }
        total / count as f64
    }

    pub fn centre_de_masse(&self) -> Option<PhaseCoordinates> {
        let n = self.coords.len();
        if n == 0 {
            return None;
        }
        let (mut s, mut b, mut nd) = (0.0, 0.0, 0.0);
        for c in &self.coords {
            s += c.signal;
            b += c.boucle;
            nd += c.noeud;
        }
        Some(PhaseCoordinates::new(
            s / n as f64,
            b / n as f64,
            nd / n as f64,
        ))
    }

    pub fn paire_plus_loin(&self) -> Option<(usize, usize, f64)> {
        let n = self.coords.len();
        if n < 2 {
            return None;
        }
        let (mut max_d, mut mi, mut mj) = (0.0, 0, 1);
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.coords[i].distance(&self.coords[j]);
                if d > max_d {
                    max_d = d;
                    mi = i;
                    mj = j;
                }
            }
        }
        Some((mi, mj, max_d))
    }

    pub fn paire_plus_proche(&self) -> Option<(usize, usize, f64)> {
        let n = self.coords.len();
        if n < 2 {
            return None;
        }
        let (mut min_d, mut mi, mut mj) = (f64::INFINITY, 0, 1);
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.coords[i].distance(&self.coords[j]);
                if d < min_d {
                    min_d = d;
                    mi = i;
                    mj = j;
                }
            }
        }
        Some((mi, mj, min_d))
    }
}

impl Default for ConvergenceMetric {
    fn default() -> Self {
        Self::new()
    }
}

/// Distance moyenne d'un point aux autres
pub fn distance_moyenne(m: &ConvergenceMetric, idx: usize) -> Option<f64> {
    let n = m.coords.len();
    if n < 2 || idx >= n {
        return None;
    }
    let total: f64 = (0..n)
        .filter(|&i| i != idx)
        .map(|i| m.coords[idx].distance(&m.coords[i]))
        .sum();
    Some(total / (n - 1) as f64)
}

/// 42 traditions majeures
pub fn traditions_42() -> ConvergenceMetric {
    let mut m = ConvergenceMetric::new();
    m.ajouter("Hermes", 0.1, 0.2, 0.7);
    m.ajouter("Trismegisme", 0.15, 0.15, 0.7);
    m.ajouter("Thoth", 0.6, 0.2, 0.2);
    m.ajouter("Prisca_Theologia", 0.2, 0.1, 0.7);
    m.ajouter("Kabbale", 0.3, 0.2, 0.5);
    m.ajouter("Judaisme", 0.5, 0.3, 0.2);
    m.ajouter("Christianisme_exo", 0.7, 0.2, 0.1);
    m.ajouter("Christianisme_eso", 0.1, 0.2, 0.7);
    m.ajouter("Islam_Soufisme", 0.3, 0.4, 0.3);
    m.ajouter("Hisba", 0.7, 0.2, 0.1);
    m.ajouter("Ummah", 0.7, 0.2, 0.1);
    m.ajouter("Zoroastrisme", 0.5, 0.3, 0.2);
    m.ajouter("Legalisme", 0.8, 0.1, 0.1);
    m.ajouter("Daoisme", 0.2, 0.3, 0.5);
    m.ajouter("Confucianisme", 0.4, 0.3, 0.3);
    m.ajouter("Bouddhisme", 0.3, 0.4, 0.3);
    m.ajouter("Bushido", 0.5, 0.3, 0.2);
    m.ajouter("Shinto", 0.4, 0.2, 0.4);
    m.ajouter("Dharma", 0.4, 0.3, 0.3);
    m.ajouter("Karma", 0.2, 0.6, 0.2);
    m.ajouter("Vietnam_Tam_Giao", 0.3, 0.3, 0.4);
    m.ajouter("Ubuntu", 0.2, 0.5, 0.3);
    m.ajouter("Yoruba", 0.3, 0.2, 0.5);
    m.ajouter("Akan", 0.3, 0.5, 0.2);
    m.ajouter("KhoiSan", 0.1, 0.8, 0.1);
    m.ajouter("Hegel", 0.2, 0.6, 0.2);
    m.ajouter("Marxisme", 0.2, 0.6, 0.2);
    m.ajouter("Rousseau", 0.3, 0.4, 0.3);
    m.ajouter("Machiavel", 0.1, 0.2, 0.7);
    m.ajouter("Nordique_Wyrd", 0.4, 0.3, 0.3);
    m.ajouter("Agnosticisme", 0.3, 0.4, 0.3);
    m.ajouter("Druidisme", 0.4, 0.2, 0.4);
    m.ajouter("Azteque", 0.4, 0.3, 0.3);
    m.ajouter("Maya", 0.4, 0.4, 0.2);
    m.ajouter("Inca_Ayni", 0.3, 0.5, 0.2);
    m.ajouter("Haudenosaunee", 0.3, 0.5, 0.2);
    m.ajouter("Tengrism", 0.5, 0.2, 0.3);
    m.ajouter("Chondogyo", 0.2, 0.2, 0.6);
    m.ajouter("Aborigene_Tjukurpa", 0.3, 0.5, 0.2);
    m.ajouter("Maori", 0.3, 0.4, 0.3);
    m.ajouter("Hawaien", 0.3, 0.4, 0.3);
    m.ajouter("CYNIC", 0.33, 0.33, 0.34);
    m
}

/// 50 traditions (48 + Hindouisme, Stoïcisme)
pub fn traditions_50() -> ConvergenceMetric {
    let mut m = traditions_48();
    m.ajouter("Hindouisme", 0.4, 0.3, 0.3);
    m.ajouter("Stoicisme", 0.5, 0.3, 0.2);
    m
}

/// 57 traditions (50 + Islam_fiqh, Tradition_masaï, Tradition_zoulou,
/// Tradition_inuit, Tradition_yanomami, Tradition_samoa, Tradition_swahili)
pub fn traditions_57() -> ConvergenceMetric {
    let mut m = traditions_50();
    // Islam institutionnel (pas seulement soufisme)
    m.ajouter("Islam_fiqh", 0.6, 0.3, 0.1);
    // Afrique de l'Ouest (complément Yoruba)
    m.ajouter("Tradition_swahili", 0.4, 0.3, 0.3);
    // Afrique australe
    m.ajouter("Tradition_zoulou", 0.3, 0.5, 0.2);
    // Afrique de l'Est
    m.ajouter("Tradition_masai", 0.3, 0.5, 0.2);
    // Arctique
    m.ajouter("Tradition_inuit", 0.3, 0.5, 0.2);
    // Amazonie
    m.ajouter("Tradition_yanomami", 0.2, 0.5, 0.3);
    // Pacifique
    m.ajouter("Tradition_samoa", 0.3, 0.4, 0.3);
    m
}

/// 45 traditions (42 + Sikhisme, Mapuche, Ecologisme)
pub fn traditions_45() -> ConvergenceMetric {
    let mut m = traditions_42();
    m.ajouter("Sikhisme", 0.4, 0.4, 0.2);
    m.ajouter("Mapuche", 0.3, 0.5, 0.2);
    m.ajouter("Ecologisme", 0.2, 0.4, 0.4);
    m
}

/// 48 traditions (45 + Open Source, Feminisme, Anarchisme)
pub fn traditions_48() -> ConvergenceMetric {
    let mut m = traditions_45();
    m.ajouter("Open_Source", 0.3, 0.4, 0.3);
    m.ajouter("Feminisme", 0.3, 0.5, 0.2);
    m.ajouter("Anarchisme", 0.2, 0.5, 0.3);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_normalisation() {
        let c = PhaseCoordinates::new(2.0, 3.0, 5.0);
        assert!((c.signal + c.boucle + c.noeud - 1.0).abs() < 1e-10);
        assert!((c.signal - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_phase_zero_default() {
        let c = PhaseCoordinates::new(0.0, 0.0, 0.0);
        assert!(c.est_valide());
        assert!((c.signal - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_distance_zero() {
        let a = PhaseCoordinates::new(0.3, 0.3, 0.4);
        let b = PhaseCoordinates::new(0.3, 0.3, 0.4);
        assert!(a.distance(&b) < 1e-10);
    }

    #[test]
    fn test_distance_max() {
        let a = PhaseCoordinates::new(1.0, 0.0, 0.0);
        let b = PhaseCoordinates::new(0.0, 1.0, 0.0);
        assert!((a.distance(&b) - 2.0f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = PhaseCoordinates::new(1.0, 0.0, 0.0);
        let b = PhaseCoordinates::new(0.0, 1.0, 0.0);
        assert!(a.cosine_similarity(&b).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_identical() {
        let c = PhaseCoordinates::new(0.3, 0.3, 0.4);
        assert!((c.cosine_similarity(&c) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_g_leontief() {
        let s = CoordinationScore::with_seuils(0.5, 0.3, 0.2, 0.1, 0.1, 0.1);
        assert!((s.g() - 2.0).abs() < 1e-10);
        assert_eq!(s.maillon_faible(), "noeud");
        assert!(s.est_viable());
    }

    #[test]
    fn test_g_maillon_signal() {
        let s = CoordinationScore::with_seuils(0.05, 0.5, 0.5, 0.1, 0.1, 0.1);
        assert_eq!(s.maillon_faible(), "signal");
        assert!(!s.est_viable());
    }

    #[test]
    fn test_g_from_coords_cynic() {
        let c = PhaseCoordinates::new(0.33, 0.33, 0.34);
        let g = g_from_coords(&c);
        assert!(g > 0.99 && g < 1.01, "G(CYNIC) devrait ≈ 1.0, got {}", g);
    }

    #[test]
    fn test_traditions_42_count() {
        let m = traditions_42();
        assert_eq!(m.len(), 42);
        assert_eq!(m.noms.len(), 42);
    }

    #[test]
    fn test_traditions_45_count() {
        let m = traditions_45();
        assert_eq!(m.len(), 45);
    }

    #[test]
    fn test_traditions_48_count() {
        let m = traditions_48();
        assert_eq!(m.len(), 48);
    }

    #[test]
    fn test_traditions_50_count() {
        let m = traditions_50();
        assert_eq!(m.len(), 50);
    }

    #[test]
    fn test_traditions_57_count() {
        let m = traditions_57();
        assert_eq!(m.len(), 57);
    }

    #[test]
    fn test_all_traditions_valides() {
        for nom in traditions_57().noms {
            let idx = traditions_48().noms.iter().position(|n| n == &nom).unwrap();
            let coord = traditions_48().coords[idx];
            assert!(coord.est_valide(), "{} invalide: {}", nom, coord);
        }
    }

    #[test]
    fn test_convergence_range() {
        let m = traditions_48();
        let c = m.convergence();
        assert!(c > 0.0 && c < 2.0, "Convergence hors range: {}", c);
    }

    #[test]
    fn test_centre_de_masse() {
        let m = traditions_48();
        let c = m.centre_de_masse().unwrap();
        assert!(c.est_valide());
        assert!((c.signal - 0.33).abs() < 0.15);
        assert!((c.boucle - 0.33).abs() < 0.15);
    }

    #[test]
    fn test_paires() {
        let mut m = ConvergenceMetric::new();
        m.ajouter("A", 1.0, 0.0, 0.0);
        m.ajouter("B", 0.0, 1.0, 0.0);
        m.ajouter("C", 0.0, 0.0, 1.0);

        let (i, j, d) = m.paire_plus_loin().unwrap();
        assert!(d > 1.9 && d < 2.1, "Distance max devrait ≈ 2.0, got {}", d);
        assert_eq!((i, j), (0, 1));
    }

    #[test]
    fn test_distance_moyenne() {
        let mut m = ConvergenceMetric::new();
        m.ajouter("A", 0.3, 0.3, 0.4);
        m.ajouter("B", 0.3, 0.3, 0.4);
        m.ajouter("C", 1.0, 0.0, 0.0);
        let d = distance_moyenne(&m, 0).unwrap();
        assert!(d > 0.0 && d < 2.0);
    }

    #[test]
    fn test_delta_g_monotone() {
        let s = CoordinationScore::new(0.5, 0.3, 0.2);
        let dg = s.delta_g(0.1, 0.0, 0.0);
        assert!(dg > 0.0, "Augmenter le signal devrait augmenter G");
    }
}
