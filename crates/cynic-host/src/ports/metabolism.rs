#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetabolicState {
    /// Le système hôte a des ressources abondantes (sur secteur, CPU au repos, bonne connexion).
    /// Le nœud peut prétendre à être Leader et exécuter des tâches lourdes.
    HighPerformance,

    /// Le système hôte est dans un état normal ou sur batterie avec une bonne charge.
    /// Le nœud participe au réseau mais limite ses appels intensifs.
    Standard,

    /// Le système hôte est en souffrance (batterie faible, surchauffe CPU, réseau instable).
    /// Le nœud passe en mode passif/survie et refuse d'être Leader.
    Survival,
}

pub trait MetabolismProvider {
    /// Retourne l'état métabolique actuel de la machine hôte.
    fn current_state(&mut self) -> MetabolicState;
}

/// Implémentation réelle du métabolisme basée sur le matériel (OS).
pub struct OsMetabolism {
    system: sysinfo::System,
}

impl OsMetabolism {
    pub fn new() -> Self {
        let mut system = sysinfo::System::new_all();
        system.refresh_all();
        Self { system }
    }
}

impl MetabolismProvider for OsMetabolism {
    fn current_state(&mut self) -> MetabolicState {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let global_cpu = self.system.global_cpu_usage();
        let free_memory = self.system.free_memory();
        let total_memory = self.system.total_memory();
        let mem_usage_percent = ((total_memory - free_memory) as f32 / total_memory as f32) * 100.0;

        // C'est ici que la logique organique s'applique :
        if global_cpu > 85.0 || mem_usage_percent > 90.0 {
            // Le PC est en train de suffoquer
            return MetabolicState::Survival;
        }

        if global_cpu > 40.0 || mem_usage_percent > 60.0 {
            // Le PC travaille déjà sur autre chose
            return MetabolicState::Standard;
        }

        // Le PC dort, on peut pomper du jus
        MetabolicState::HighPerformance
    }
}
