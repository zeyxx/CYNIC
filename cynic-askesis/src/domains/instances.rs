use chrono::NaiveTime;
use super::DomainTracker;

pub struct Body;
impl DomainTracker for Body {
    fn name(&self) -> &str { "body" }
    fn log_prompt(&self) -> &str { "Qu'est-ce qui est vrai aujourd'hui sur ton corps ?" }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "Tiens-tu tes promesses à toi-même ? (Authenticité)",
            "Patterns de self-deception vs reporting honnête ?",
            "KENOSIS: Qu'as-tu arrêté de faire subir à ton corps ?",
            "PHI: Équilibre entre effort et repos (Recovery chemical/neuro) ?"
        ]
    }
    fn anchor_time(&self) -> NaiveTime { NaiveTime::from_hms_opt(19, 0, 0).unwrap() }
}

pub struct Solana;
impl DomainTracker for Solana {
    fn name(&self) -> &str { "solana" }
    fn log_prompt(&self) -> &str { "Solana KPI : Qu'as-tu construit ou vérifié sur la chain aujourd'hui ?" }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "Impact réel vs activité de surface (Poiesis vs Praxis) ?",
            "Souveraineté technique : es-tu dépendant d'infra tierce ?",
            "FIDELITY: Est-ce que tes métriques mentent ?",
            "CULTURE: Comment ton travail honore-t-il l'ethos de la chain ?"
        ]
    }
    fn anchor_time(&self) -> NaiveTime { NaiveTime::from_hms_opt(18, 30, 0).unwrap() }
}

pub struct Reading;
impl DomainTracker for Reading {
    fn name(&self) -> &str { "reading" }
    fn log_prompt(&self) -> &str { "Qu'as-tu lu aujourd'hui et qu'en as-tu retenu ?" }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "Profondeur vs Défilement : as-tu réellement 'digéré' ?",
            "BURN: Était-ce une consommation de bruit ou de substance ?",
            "CULTURE: Quel lien avec tes traditions de pensée ?"
        ]
    }
    fn anchor_time(&self) -> NaiveTime { NaiveTime::from_hms_opt(22, 0, 0).unwrap() }
}

pub struct Sovereignty;
impl DomainTracker for Sovereignty {
    fn name(&self) -> &str { "sovereignty" }
    fn log_prompt(&self) -> &str { "Attention Audit : Qui a dirigé ton focus aujourd'hui ?" }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "Captivité : Quels moments ont été volés par les plateformes ?",
            "PHI: Harmonie entre ton intention matinale et tes actes ?",
            "PROJECTION: As-tu agi malgré la peur du 'bug Erin' ?",
            "SOVEREIGNTY: Es-tu resté vertical pendant l'interaction ?"
        ]
    }
    fn anchor_time(&self) -> NaiveTime { NaiveTime::from_hms_opt(21, 0, 0).unwrap() }
}
