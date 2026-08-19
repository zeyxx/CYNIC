use serde::{Deserialize, Serialize};

/// Schema pour les enveloppes temporelles CYNIC.
pub const TEMPORAL_EPOCH_CERTIFICATE_SCHEMA: &str = "cynic.temporal.epoch_certificate.v1";
pub const TEMPORAL_SNAPSHOT_SCHEMA: &str = "cynic.temporal.snapshot.v1";
pub const TEMPORAL_PROOF_SCHEMA: &str = "cynic.temporal.proof.v1";

/// Un identifiant d'epoch dans le système CYNIC.
///
/// Les epochs sont des unités discrètes de temps logique dans le runtime.
/// Chaque epoch a une frontière bien définie et peut être utilisée comme
/// point de référence pour la convergence, le rollback, ou la preuve.
///
/// Contrairement à un timestamp absolu, un epoch est incrémental et
/// ne dépend que de la progression du runtime, pas du temps physique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Epoch(pub u64);

impl Epoch {
    /// L'epoch zéro (départ du runtime).
    pub const ZERO: Epoch = Epoch(0);

    /// Crée un nouvel epoch depuis un identifiant u64.
    pub const fn new(id: u64) -> Self {
        Epoch(id)
    }

    /// Incrémente l'epoch (retourne l'epoch suivant).
    pub fn next(self) -> Epoch {
        Epoch(self.0 + 1)
    }

    /// Décrémente l'epoch (pour rollback), sans aller sous zéro.
    pub fn prev(self) -> Epoch {
        if self.0 == 0 {
            Epoch(0)
        } else {
            Epoch(self.0 - 1)
        }
    }

    /// La valeur numérique de l'epoch.
    pub fn id(self) -> u64 {
        self.0
    }
}

impl From<u64> for Epoch {
    fn from(id: u64) -> Self {
        Epoch(id)
    }
}

impl Default for Epoch {
    fn default() -> Self {
        Epoch::ZERO
    }
}

/// Une preuve temporelle associée à un epoch.
///
/// Cette preuve garantit que l'état observé est valide à l'epoch spécifié.
/// Elle peut être utilisée pour valider la convergence ou déclencher un rollback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalProof {
    pub schema: String,
    /// L'epoch auquel cette preuve est valide.
    pub epoch: Epoch,
    /// L'epoch parent (precédent) auquel l'état avait convergé.
    pub base_epoch: Option<Epoch>,
    /// Le digest de l'état observé à cet epoch.
    pub state_digest: String,
    /// Les preuves d'observation sous-jacentes.
    pub evidence: Vec<String>,
    /// Horodatage de création (epoch logique).
    pub observed_at_epoch: Epoch,
}

impl TemporalProof {
    pub fn new(epoch: Epoch, state_digest: impl Into<String>) -> Self {
        Self {
            schema: TEMPORAL_PROOF_SCHEMA.to_string(),
            epoch,
            base_epoch: None,
            state_digest: state_digest.into(),
            evidence: Vec::new(),
            observed_at_epoch: epoch,
        }
    }

    /// Ajoute une preuve de base (epoch parent).
    pub fn with_base_epoch(mut self, base: Epoch) -> Self {
        self.base_epoch = Some(base);
        self
    }

    /// Ajoute des preuves d'observation.
    pub fn with_evidence(mut self, evidence: Vec<String>) -> Self {
        self.evidence = evidence;
        self
    }
}

/// Un snapshot d'état à un epoch donné.
///
/// Ce snapshot peut être utilisé pour :
/// - La récupération après échec (rollback)
/// - La vérification de convergence
/// - L'audit temporel
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalSnapshot {
    pub schema: String,
    pub epoch: Epoch,
    /// Digest de l'état complet au snapshot.
    pub state_digest: String,
    /// Références aux preuves d'observation.
    pub proof_refs: Vec<String>,
    /// Taille approximative du snapshot (mémoire/disk).
    pub size_hint_bytes: u64,
}

impl TemporalSnapshot {
    pub fn new(epoch: Epoch, state_digest: impl Into<String>) -> Self {
        Self {
            schema: TEMPORAL_SNAPSHOT_SCHEMA.to_string(),
            epoch,
            state_digest: state_digest.into(),
            proof_refs: Vec::new(),
            size_hint_bytes: 0,
        }
    }

    /// Lie des preuves au snapshot.
    pub fn with_proofs(mut self, proofs: Vec<String>) -> Self {
        self.proof_refs = proofs;
        self
    }

    /// Estime la taille du snapshot.
    pub fn with_size_hint(mut self, size: u64) -> Self {
        self.size_hint_bytes = size;
        self
    }
}

/// Un certificat d'epoch.
///
/// Ce certificat atteste qu'un nœud a observé et validé un snapshot
/// à un epoch donné. Il peut être gossippé dans le réseau P2P
/// pour synchroniser les nœuds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochCertificate {
    pub schema: String,
    /// L'epoch certifié.
    pub epoch: Epoch,
    /// Le nœud qui a émis ce certificat.
    pub issuer_node_id: String,
    /// La signature cryptographique du nœud émetteur.
    pub signature: String,
    /// Le digest du snapshot certifié.
    pub snapshot_digest: String,
    /// Les epochs de base (ancêtres) nécessaires pour valider ce certificat.
    pub base_epochs: Vec<Epoch>,
}

impl EpochCertificate {
    pub fn new(
        epoch: Epoch,
        issuer_node_id: impl Into<String>,
        signature: impl Into<String>,
        snapshot_digest: impl Into<String>,
    ) -> Self {
        Self {
            schema: TEMPORAL_EPOCH_CERTIFICATE_SCHEMA.to_string(),
            epoch,
            issuer_node_id: issuer_node_id.into(),
            signature: signature.into(),
            snapshot_digest: snapshot_digest.into(),
            base_epochs: vec![epoch.prev()],
        }
    }

    /// Ajoute des epochs de base supplémentaires.
    pub fn with_base_epochs(mut self, epochs: Vec<Epoch>) -> Self {
        self.base_epochs = epochs;
        self
    }
}

/// Contrat de vertical temporel.
///
/// Ce contrat définit la couche d'abstraction pour la notion de temps
/// dans le runtime CYNIC. Il formalise la manière dont les nœuds doivent
/// se synchroniser temporellement, comment ils produisent des preuves
/// temporelles, et quelles sont les conditions de rollback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalContract {
    pub schema: &'static str,
    pub id: &'static str,
    pub desired_state: &'static str,
    pub observer: &'static str,
    pub reconciliation: &'static str,
    pub bounded_action: &'static str,
    pub read_back_proof: &'static str,
    pub owner: Vec<&'static str>,
    pub coverage: &'static str,
}

impl TemporalContract {
    /// Schéma de version pour le contrat temporel.
    pub const TEMPORAL_CONTRACT_SCHEMA: &'static str = "cynic.temporal_contract.v1";

    /// Version statique du contrat temporel.
    ///
    /// Ce contrat est conçu pour unifier les notions d'epoch, de snapshot,
    /// et de rollback à travers l'ensemble des verticales du runtime.
    ///
    /// NOTE: pas de `const` ici — `Vec` n'est pas const-constructible.
    /// `vec!` macro inside const déclenche ICE rustc 1.95 (box_assume_init
    /// non-const). Utiliser `contract()` function à la place.
    pub fn contract() -> TemporalContract {
        TemporalContract {
            schema: Self::TEMPORAL_CONTRACT_SCHEMA,
            id: "temporal",
            desired_state: "consistent epoch progression, verifiable checkpoints, and bounded rollback windows",
            observer: "epoch clock probe, checkpoint observer, temporal snapshot producer",
            reconciliation: "admit only certificate-backed epoch advancement; request rollback on divergence",
            bounded_action: "accept epoch certificate, request rollback to certified base_epoch, or emit new snapshot",
            read_back_proof: "epoch certificate, temporal proof bundle, state digest match",
            owner: vec!["cynic-protocol", "cynic-kernel", "cynic-node"],
            coverage: "partial",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_progression_is_monotonic() {
        let e0 = Epoch::ZERO;
        let e1 = e0.next();
        let e2 = e1.next();

        assert!(e0 < e1);
        assert!(e1 < e2);
        assert_eq!(e1, Epoch::new(1));
        assert_eq!(e2, Epoch::new(2));
    }

    #[test]
    fn epoch_prev_is_bounded_at_zero() {
        let e0 = Epoch::ZERO;
        assert_eq!(e0.prev(), Epoch::ZERO); // Ne descend pas sous zéro

        let e5 = Epoch::new(5);
        assert_eq!(e5.prev(), Epoch::new(4));
    }

    #[test]
    fn epoch_default_is_zero() {
        assert_eq!(Epoch::default(), Epoch::ZERO);
    }

    #[test]
    fn temporal_proof_serializes_with_correct_schema() {
        let proof = TemporalProof::new(Epoch::new(3), "sha256:abc123");
        assert_eq!(proof.schema, TEMPORAL_PROOF_SCHEMA);
        assert_eq!(proof.epoch, Epoch::new(3));
        assert_eq!(proof.state_digest, "sha256:abc123");
    }

    #[test]
    fn temporal_snapshot_links_to_epoch() {
        let snap = TemporalSnapshot::new(Epoch::new(7), "sha256:def456")
            .with_proofs(vec!["proof-1".to_string()])
            .with_size_hint(1024);

        assert_eq!(snap.epoch, Epoch::new(7));
        assert_eq!(snap.proof_refs.len(), 1);
        assert_eq!(snap.size_hint_bytes, 1024);
    }

    #[test]
    fn epoch_certificate_has_valid_base_epochs() {
        let cert = EpochCertificate::new(
            Epoch::new(5),
            "node-1",
            "sig-xyz",
            "sha256:snap5",
        );

        assert_eq!(cert.epoch, Epoch::new(5));
        assert_eq!(cert.base_epochs, vec![Epoch::new(4)]); // Parent par défaut
    }

    #[test]
    fn temporal_contract_declares_coverage_and_owner() {
        let contract = TemporalContract::contract();
        assert_eq!(contract.id, "temporal");
        assert!(contract.coverage == "partial" || contract.coverage == "complete");
        assert!(!contract.owner.is_empty());
        assert!(contract.reconciliation.contains("certificate"));
    }
}
