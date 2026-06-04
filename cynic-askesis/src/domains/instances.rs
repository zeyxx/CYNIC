use super::DomainTracker;
use chrono::NaiveTime;

pub(super) struct Body;
impl DomainTracker for Body {
    fn name(&self) -> &str {
        "body"
    }
    fn log_prompt(&self) -> &str {
        "Qu'est-ce qui est vrai aujourd'hui sur ton corps ?"
    }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "Tiens-tu tes promesses à toi-même ? (Authenticité)",
            "Patterns de self-deception vs reporting honnête ?",
            "KENOSIS: Qu'as-tu arrêté de faire subir à ton corps ?",
            "PHI: Équilibre entre effort et repos (Recovery chemical/neuro) ?",
        ]
    }
    fn anchor_time(&self) -> NaiveTime {
<<<<<<< HEAD
        NaiveTime::from_hms_opt(19, 0, 0).unwrap_or_default()
=======
<<<<<<< HEAD
        NaiveTime::from_hms_opt(19, 0, 0).unwrap()
=======
        NaiveTime::from_hms_opt(19, 0, 0).unwrap_or_default()
>>>>>>> origin/main
>>>>>>> origin/main
    }
}

pub(super) struct Solana;
impl DomainTracker for Solana {
    fn name(&self) -> &str {
        "solana"
    }
    fn log_prompt(&self) -> &str {
        "Solana KPI : Qu'as-tu construit ou vérifié sur la chain aujourd'hui ?"
    }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "Impact réel vs activité de surface (Poiesis vs Praxis) ?",
            "Souveraineté technique : es-tu dépendant d'infra tierce ?",
            "FIDELITY: Est-ce que tes métriques mentent ?",
            "CULTURE: Comment ton travail honore-t-il l'ethos de la chain ?",
        ]
    }
    fn anchor_time(&self) -> NaiveTime {
<<<<<<< HEAD
        NaiveTime::from_hms_opt(18, 30, 0).unwrap_or_default()
=======
<<<<<<< HEAD
        NaiveTime::from_hms_opt(18, 30, 0).unwrap()
=======
        NaiveTime::from_hms_opt(18, 30, 0).unwrap_or_default()
>>>>>>> origin/main
>>>>>>> origin/main
    }
}

pub(super) struct Reading;
impl DomainTracker for Reading {
    fn name(&self) -> &str {
        "reading"
    }
    fn log_prompt(&self) -> &str {
        "Qu'as-tu lu aujourd'hui et qu'en as-tu retenu ?"
    }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "Profondeur vs Défilement : as-tu réellement 'digéré' ?",
            "BURN: Était-ce une consommation de bruit ou de substance ?",
            "CULTURE: Quel lien avec tes traditions de pensée ?",
        ]
    }
    fn anchor_time(&self) -> NaiveTime {
<<<<<<< HEAD
        NaiveTime::from_hms_opt(22, 0, 0).unwrap_or_default()
=======
<<<<<<< HEAD
        NaiveTime::from_hms_opt(22, 0, 0).unwrap()
=======
        NaiveTime::from_hms_opt(22, 0, 0).unwrap_or_default()
>>>>>>> origin/main
>>>>>>> origin/main
    }
}

pub(super) struct Sovereignty;
impl DomainTracker for Sovereignty {
    fn name(&self) -> &str {
        "sovereignty"
    }
    fn log_prompt(&self) -> &str {
        "Attention Audit : Qui a dirigé ton focus aujourd'hui ?"
    }
    fn audit_questions(&self) -> Vec<&str> {
        vec![
            "OXYTOCINE: As-tu ressenti la 'faim' du lien aujourd'hui ?",
            "COÛT: As-tu évité l'action par peur de l'abandon de soi ?",
            "WRITE_ACCESS: Es-tu resté maître de ton code source face à l'autre ?",
            "SOVEREIGNTY: Ta verticalité a-t-elle survécu à la proximité ?",
        ]
    }
    fn anchor_time(&self) -> NaiveTime {
<<<<<<< HEAD
        NaiveTime::from_hms_opt(21, 0, 0).unwrap_or_default()
=======
<<<<<<< HEAD
        NaiveTime::from_hms_opt(21, 0, 0).unwrap()
=======
        NaiveTime::from_hms_opt(21, 0, 0).unwrap_or_default()
>>>>>>> origin/main
>>>>>>> origin/main
    }
}
