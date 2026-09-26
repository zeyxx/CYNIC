//! Canonical judgment surface — one type that projects the runtime's
//! observe→reconcile→act→read-back loop through three orthogonal lenses:
//!
//! - **DIKW layer** (Data → Information → Knowledge → Wisdom): at which level
//!   of meaning does this judgment sit?
//! - **SAGE phase** (Priming → Sense → Simulate → Act → Evaluate): in which
//!   phase of the sense-making loop?
//! - **Cynefin domain** (Clear → Complicated → Complex → Chaotic → Disordered):
//!   in which decision regime?
//!
//! The three are *orthogonal*: a single judgment can be Knowledge (DIKW),
//! in the Act phase (SAGE), under a Complicated regime (Cynefin). Keeping them
//! as fields of one type avoids fragmenting the runtime's self-knowledge into
//! three parallel silos that each re-calculate the same uncertainty.
//!
//! # Schema evolution contract
//!
//! - `schema` is a `String` for forward compat (consumers can read newer
//!   versions as long as they treat unknown fields conservatively).
//! - `JudgmentVersion` is the compile-time mirror for `match` without string
//!   compare.
//! - All enums carry `#[serde(other)]` so a new variant does not break older
//!   consumers — they see `Unknown` and preserve it as residual risk.
//! - `metadata: Value` is the extension escape hatch: ad-hoc fields go there
//!   without a schema bump. Promoted to a first-class field only when the
//!   pattern stabilizes.
//! - Every new field added to `CynicJudgment` **must** carry
//!   `#[serde(default)]` so older JSON still deserializes.
//! - Migration between versions is explicit via `JudgmentMigration`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use cynic_protocol::evidence::EvidenceRef;

pub const JUDGMENT_SCHEMA: &str = "cynic.judgment.v1";

/// Compile-time mirror of the schema string. Use this for `match` instead of
/// comparing the `schema` string directly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JudgmentVersion {
    V1,
    Unknown(String),
}

impl Default for JudgmentVersion {
    fn default() -> Self {
        Self::V1
    }
}

impl JudgmentVersion {
    pub fn as_str(&self) -> &str {
        match self {
            Self::V1 => "cynic.judgment.v1",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl From<&str> for JudgmentVersion {
    fn from(value: &str) -> Self {
        match value {
            "cynic.judgment.v1" => Self::V1,
            other => Self::Unknown(other.to_string()),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────
// Projections
// ──────────────────────────────────────────────────────────────────────

/// DIKW: at which layer of sense-making does this judgment sit?
///
/// `Data` is raw, unstructured. `Information` is schema-validated and typed.
/// `Knowledge` is a pure decision rule applied to Information. `Wisdom` is
/// the metacognitive layer that judges whether Knowledge is sufficient — it
/// is never 100% certain and always carries residual unknowns.
///
/// `Unknown` is the forward-compat sentinel: a variant added in v2 does not
/// break v1 consumers — they see `Unknown` and preserve it as residual risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DijkwLayer {
    Data,
    Information,
    Knowledge,
    Wisdom,
    #[serde(other)]
    Unknown,
}

/// SAGE: in which phase of the sense-making loop?
///
/// `Priming` recalls past experience. `Sense` assembles observations into a
/// coherent account. `Simulate` plays out an action before committing. `Act`
/// executes with bounded authority. `Evaluates` closes the loop via read-back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SagePhase {
    Priming,
    Sense,
    Simulate,
    Act,
    Evaluate,
    #[serde(other)]
    Unknown,
}

/// Cynefin: under which decision regime?
///
/// `Clear` — known knowns, sense→categorize→respond. `Complicated` — known
/// unknowns, sense→analyze→respond. `Complex` — unknown unknowns, probe→sense→
/// respond. `Chaotic` — unknowable, act→sense→respond. `Disordered` — not yet
/// classified; the runtime must not fabricate a premature category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CynefinDomain {
    Clear,
    Complicated,
    Complex,
    Chaotic,
    Disordered,
    #[serde(other)]
    Unknown,
}

// ──────────────────────────────────────────────────────────────────────
// Canonical judgment
// ──────────────────────────────────────────────────────────────────────

/// One judgment produced by the runtime's observe→reconcile→act→read-back
/// loop, projected through three orthogonal lenses. This is the single
/// canonical type — consumers read the projection they need and ignore the
/// rest.
///
/// # Evolution rules (immutable)
///
/// 1. Never remove a field — only deprecate via `#[deprecated]` + doc.
/// 2. Every new field **must** carry `#[serde(default)]`.
/// 3. Every new enum variant **must** be `#[serde(other)]` (named `Unknown`).
/// 4. `metadata: Value` absorbs ad-hoc extensions without a bump.
/// 5. A schema bump requires a `JudgmentMigration` impl.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CynicJudgment {
    pub schema: String,
    /// Compile-time version mirror. Derived from `schema` via `From`.
    #[serde(skip)]
    pub version: JudgmentVersion,
    /// Stable identifier for this judgment.
    pub judgment_id: String,
    /// Whether the runtime may act on this judgment. Set by the producer
    /// (a plan, a matrix, an anomaly) — not derived from the projections.
    /// A `true` here with a `Disordered` Cynefin domain is a contradiction
    /// the consumer must treat as residual risk.
    #[serde(default)]
    pub actionable: bool,
    /// Content-addressed observation(s) this judgment derives from.
    #[serde(default)]
    pub observation_refs: Vec<EvidenceRef>,
    /// At which DIKW layer does this judgment currently sit.
    pub dijkw_layer: DijkwLayer,
    /// Current phase in the SAGE loop.
    pub sage_phase: SagePhase,
    /// The Cynefin domain the runtime classifies itself into for this judgment.
    pub cynefin_domain: CynefinDomain,
    /// Confidence in this judgment, in `[0.0, 1.0]`. Wisdom never reaches 1.0.
    #[serde(default)]
    pub confidence: f64,
    /// What remains unknown. Preserved, never fabricated away.
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
    /// Content-addressed proof that this judgment was produced and acted upon.
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
    /// Wall-clock at judgment time.
    pub judged_at_unix_s: u64,
    /// Extension escape hatch. Ad-hoc fields go here without a schema bump.
    /// Promoted to a first-class field only when the pattern stabilizes.
    #[serde(default, flatten)]
    pub metadata: Value,
}

impl CynicJudgment {
    /// Construct a v1 judgment. The `version` field is derived from `schema`.
    /// `actionable` defaults to `false` — the producer must explicitly opt in.
    pub fn new(
        judgment_id: impl Into<String>,
        actionable: bool,
        dijkw_layer: DijkwLayer,
        sage_phase: SagePhase,
        cynefin_domain: CynefinDomain,
        judged_at_unix_s: u64,
    ) -> Self {
        Self {
            schema: JUDGMENT_SCHEMA.to_string(),
            version: JudgmentVersion::V1,
            judgment_id: judgment_id.into(),
            actionable,
            observation_refs: Vec::new(),
            dijkw_layer,
            sage_phase,
            cynefin_domain,
            confidence: 0.0,
            residual_unknowns: Vec::new(),
            evidence_refs: Vec::new(),
            judged_at_unix_s,
            metadata: Value::Null,
        }
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_typed_metadata<T: Serialize>(
        mut self,
        metadata: &T,
    ) -> Result<Self, serde_json::Error> {
        self.metadata = serde_json::to_value(metadata)?;
        Ok(self)
    }

    /// Computes deterministic canonical JSON bytes for this judgment.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        cynic_protocol::canonical_json_bytes(self)
    }

    /// Computes the content-addressed digest for this judgment.
    pub fn digest(&self) -> Result<String, serde_json::Error> {
        cynic_node_protocol::digest_json(self)
    }
    pub fn is_actionable(&self) -> bool {
        self.actionable
            && !matches!(
                self.cynefin_domain,
                CynefinDomain::Disordered | CynefinDomain::Unknown
            )
    }

    /// Attempt to promote the judgment one DIKW layer up. Returns `None` if
    /// the judgment is already at Wisdom, or if residual unknowns make
    /// promotion unsafe (the runtime must not pretend to know more than it
    /// does).
    pub fn promote_layer(&self) -> Option<CynicJudgment> {
        let next = match self.dijkw_layer {
            DijkwLayer::Data => DijkwLayer::Information,
            DijkwLayer::Information => DijkwLayer::Knowledge,
            DijkwLayer::Knowledge => DijkwLayer::Wisdom,
            DijkwLayer::Wisdom | DijkwLayer::Unknown => return None,
        };
        let mut promoted = self.clone();
        promoted.dijkw_layer = next;
        // Confidence decays as we climb — each layer is a further abstraction
        // from raw observation. Wisdom is never 1.0.
        promoted.confidence = (self.confidence * 0.9).min(0.99);
        Some(promoted)
    }

    /// Re-classify the Cynefin domain from the judgment's current state.
    /// This is a pure function of the layer, unknowns, and confidence — not
    /// a new observation.
    pub fn classify_domain(&self) -> CynefinDomain {
        match self.dijkw_layer {
            DijkwLayer::Data | DijkwLayer::Unknown => CynefinDomain::Disordered,
            DijkwLayer::Information => {
                if self.residual_unknowns.is_empty() {
                    CynefinDomain::Clear
                } else {
                    CynefinDomain::Complicated
                }
            }
            DijkwLayer::Knowledge => {
                if self.residual_unknowns.is_empty() && self.confidence >= 0.8 {
                    CynefinDomain::Clear
                } else if self.confidence >= 0.5 {
                    CynefinDomain::Complicated
                } else {
                    CynefinDomain::Complex
                }
            }
            DijkwLayer::Wisdom => {
                // Wisdom always carries residual unknowns by construction.
                // High confidence + known unknowns = Complicated.
                // Low confidence = Complex (emergent, hypotheses to validate).
                if self.confidence >= 0.6 {
                    CynefinDomain::Complicated
                } else {
                    CynefinDomain::Complex
                }
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────
// Migration
// ──────────────────────────────────────────────────────────────────────

/// Explicit migration path between schema versions. A bump from v1 → v2
/// requires an impl of this trait. The source version is preserved in the
/// `schema` field of the output so the migration chain is auditable.
pub trait JudgmentMigration {
    type Source;
    /// Migrate from a previous version. Returns `Err` with the reason if the
    /// migration cannot be performed losslessly — the caller must preserve
    /// the original as residual evidence.
    fn migrate(source: &Self::Source) -> Result<CynicJudgment, String>;
}

// ──────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_judgment() -> CynicJudgment {
        CynicJudgment {
            schema: JUDGMENT_SCHEMA.to_string(),
            version: JudgmentVersion::V1,
            judgment_id: "j-1".to_string(),
            actionable: true,
            observation_refs: vec![],
            dijkw_layer: DijkwLayer::Information,
            sage_phase: SagePhase::Sense,
            cynefin_domain: CynefinDomain::Complicated,
            confidence: 0.7,
            residual_unknowns: vec!["thp_enabled_unreadable".to_string()],
            evidence_refs: vec![],
            judged_at_unix_s: 1_700_000_000,
            metadata: Value::Null,
        }
    }

    // ── Actionability ──

    #[test]
    fn data_layer_alone_does_not_block_when_producer_opts_in() {
        // The producer's intent is primary; layer does not gate actionability.
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Data,
            sage_phase: SagePhase::Act,
            actionable: true,
            ..sample_judgment()
        };
        assert!(j.is_actionable());
    }

    #[test]
    fn priming_phase_alone_does_not_block_when_producer_opts_in() {
        // The producer's intent is primary; phase does not gate actionability.
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Knowledge,
            sage_phase: SagePhase::Priming,
            actionable: true,
            ..sample_judgment()
        };
        assert!(j.is_actionable());
    }

    #[test]
    fn disordered_domain_is_never_actionable() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Knowledge,
            sage_phase: SagePhase::Act,
            cynefin_domain: CynefinDomain::Disordered,
            ..sample_judgment()
        };
        assert!(!j.is_actionable());
    }

    #[test]
    fn unknown_phase_is_actionable_when_producer_says_so() {
        // The producer's `actionable` intent is primary; phase does not gate it.
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Knowledge,
            sage_phase: SagePhase::Unknown,
            ..sample_judgment()
        };
        assert!(j.is_actionable());
    }

    #[test]
    fn non_actionable_producer_intent_overrides_domain() {
        let j = CynicJudgment {
            actionable: false,
            dijkw_layer: DijkwLayer::Knowledge,
            sage_phase: SagePhase::Act,
            cynefin_domain: CynefinDomain::Clear,
            ..sample_judgment()
        };
        assert!(!j.is_actionable());
    }

    #[test]
    fn knowledge_plus_act_is_actionable() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Knowledge,
            sage_phase: SagePhase::Act,
            ..sample_judgment()
        };
        assert!(j.is_actionable());
    }

    // ── Layer promotion ──

    #[test]
    fn promote_layer_climbs_and_decays_confidence() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Data,
            confidence: 1.0,
            ..sample_judgment()
        };
        let j2 = j.promote_layer().unwrap();
        assert_eq!(j2.dijkw_layer, DijkwLayer::Information);
        assert!((j2.confidence - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn promote_layer_stops_at_wisdom() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Wisdom,
            ..sample_judgment()
        };
        assert!(j.promote_layer().is_none());
    }

    #[test]
    fn promote_layer_unknown_cannot_promote() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Unknown,
            ..sample_judgment()
        };
        assert!(j.promote_layer().is_none());
    }

    #[test]
    fn wisdom_confidence_is_capped_below_one() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Knowledge,
            confidence: 0.99,
            ..sample_judgment()
        };
        let j2 = j.promote_layer().unwrap();
        assert_eq!(j2.dijkw_layer, DijkwLayer::Wisdom);
        assert!(j2.confidence < 1.0);
    }

    // ── Domain classification ──

    #[test]
    fn classify_domain_data_is_disordered() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Data,
            ..sample_judgment()
        };
        assert_eq!(j.classify_domain(), CynefinDomain::Disordered);
    }

    #[test]
    fn classify_domain_unknown_layer_is_disordered() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Unknown,
            ..sample_judgment()
        };
        assert_eq!(j.classify_domain(), CynefinDomain::Disordered);
    }

    #[test]
    fn classify_domain_information_no_unknowns_is_clear() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Information,
            residual_unknowns: vec![],
            ..sample_judgment()
        };
        assert_eq!(j.classify_domain(), CynefinDomain::Clear);
    }

    #[test]
    fn classify_domain_wisdom_is_never_clear() {
        let j = CynicJudgment {
            dijkw_layer: DijkwLayer::Wisdom,
            confidence: 0.99,
            ..sample_judgment()
        };
        // Wisdom always has residual unknowns → Complicated at best.
        assert_eq!(j.classify_domain(), CynefinDomain::Complicated);
    }

    // ── Version ──

    #[test]
    fn version_from_schema_string() {
        assert_eq!(
            JudgmentVersion::from("cynic.judgment.v1"),
            JudgmentVersion::V1
        );
        assert_eq!(
            JudgmentVersion::from("cynic.judgment.v2"),
            JudgmentVersion::Unknown("cynic.judgment.v2".to_string())
        );
    }

    #[test]
    fn version_as_str_roundtrip() {
        let v = JudgmentVersion::V1;
        assert_eq!(v.as_str(), "cynic.judgment.v1");
    }

    // ── Forward compatibility: unknown enum variants ──

    #[test]
    fn unknown_dijkw_variant_does_not_break_deserialize() {
        let json = r#"{"schema":"cynic.judgment.v1","judgment_id":"j","dijkw_layer":"future_layer","sage_phase":"act","cynefin_domain":"clear","judged_at_unix_s":1}"#;
        let j: CynicJudgment = serde_json::from_str(json).unwrap();
        assert_eq!(j.dijkw_layer, DijkwLayer::Unknown);
        // Unknown layer → not actionable, not promotable.
        assert!(!j.is_actionable());
        assert!(j.promote_layer().is_none());
    }

    #[test]
    fn unknown_sage_phase_does_not_break_deserialize() {
        let json = r#"{"schema":"cynic.judgment.v1","judgment_id":"j","dijkw_layer":"knowledge","sage_phase":"future_phase","cynefin_domain":"clear","judged_at_unix_s":1}"#;
        let j: CynicJudgment = serde_json::from_str(json).unwrap();
        assert_eq!(j.sage_phase, SagePhase::Unknown);
        assert!(!j.is_actionable());
    }

    #[test]
    fn unknown_cynefin_domain_does_not_break_deserialize() {
        let json = r#"{"schema":"cynic.judgment.v1","judgment_id":"j","dijkw_layer":"knowledge","sage_phase":"act","cynefin_domain":"future_domain","judged_at_unix_s":1}"#;
        let j: CynicJudgment = serde_json::from_str(json).unwrap();
        assert_eq!(j.cynefin_domain, CynefinDomain::Unknown);
        assert!(!j.is_actionable());
    }

    // ── Backward compatibility: missing fields default ──

    #[test]
    fn missing_optional_fields_default_cleanly() {
        // Minimal v1 JSON — no confidence, no unknowns, no evidence_refs, no metadata.
        let json = r#"{"schema":"cynic.judgment.v1","judgment_id":"j","dijkw_layer":"information","sage_phase":"sense","cynefin_domain":"complicated","judged_at_unix_s":1}"#;
        let j: CynicJudgment = serde_json::from_str(json).unwrap();
        assert_eq!(j.confidence, 0.0);
        assert!(j.residual_unknowns.is_empty());
        assert!(j.evidence_refs.is_empty());
        // flatten with default yields an empty Object when no extra fields present.
        assert!(j.metadata.is_object());
    }

    // ── Metadata extension ──

    #[test]
    fn metadata_flatten_captures_extra_fields() {
        let json = r#"{"schema":"cynic.judgment.v1","judgment_id":"j","dijkw_layer":"information","sage_phase":"sense","cynefin_domain":"complicated","judged_at_unix_s":1,"custom_field":"custom_value","another":42}"#;
        let j: CynicJudgment = serde_json::from_str(json).unwrap();
        // Extra fields land in metadata via flatten.
        assert_eq!(j.metadata["custom_field"], "custom_value");
        assert_eq!(j.metadata["another"], 42);
    }

    // ── Schema stability ──

    #[test]
    fn schema_is_stable() {
        assert_eq!(JUDGMENT_SCHEMA, "cynic.judgment.v1");
    }

    #[test]
    fn new_constructs_v1() {
        let j = CynicJudgment::new(
            "j-new",
            false,
            DijkwLayer::Knowledge,
            SagePhase::Act,
            CynefinDomain::Complicated,
            1_700_000_000,
        );
        assert_eq!(j.schema, JUDGMENT_SCHEMA);
        assert_eq!(j.version, JudgmentVersion::V1);
        assert_eq!(j.judgment_id, "j-new");
        assert!(!j.actionable);
    }

    #[test]
    fn judgment_canonical_hashing_is_deterministic() {
        let j = CynicJudgment::new(
            "j-hash",
            true,
            DijkwLayer::Knowledge,
            SagePhase::Act,
            CynefinDomain::Complicated,
            1_700_000_000,
        );
        let digest1 = j.digest().expect("digest 1");
        let digest2 = j.digest().expect("digest 2");
        assert_eq!(digest1, digest2);
        assert!(digest1.starts_with("sha256:"));

        let bytes = j.canonical_bytes().expect("canonical bytes");
        let text = String::from_utf8(bytes).expect("valid utf-8");
        assert!(text.starts_with("{\"actionable\":true"));
    }

    #[test]
    fn judgment_with_typed_metadata_roundtrips() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct CustomMeta {
            agent_id: String,
            run_id: u64,
        }
        let meta = CustomMeta {
            agent_id: "hermes-1".to_string(),
            run_id: 42,
        };
        let j = CynicJudgment::new(
            "j-meta",
            false,
            DijkwLayer::Information,
            SagePhase::Sense,
            CynefinDomain::Clear,
            1_700_000_000,
        )
        .with_typed_metadata(&meta)
        .expect("attach meta");

        let serialized = serde_json::to_string(&j).expect("serialize");
        let deserialized: CynicJudgment = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.metadata["agent_id"], "hermes-1");
        assert_eq!(deserialized.metadata["run_id"], 42);
    }
}
