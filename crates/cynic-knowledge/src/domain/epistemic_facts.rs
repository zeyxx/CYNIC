//! Epistemic Facts, Verifiable Claims & Multi-Repo Provenance
//!
//! Schema: `cynic.facts.live.v1`
//!
//! Defines the normative domain model for machine-verifiable assertions,
//! 5-tier claim status categorization (demonstrated, implemented, partial, aspirational, unknown),
//! calibrated skills (exercised, understood, exposition), and multi-era repository provenance.

use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const EPISTEMIC_FACTS_SCHEMA_V1: &str = "cynic.facts.live.v1";
pub const INTERACTIONS_GRAPH_SCHEMA_V1: &str = "cynic.interactions_graph.v1";

/// Kind of external or ecosystem interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionCategory {
    /// Upstream compiler or runtime bug report (e.g. rust-lang/rust LLVM FastISel SIGSEGV).
    CompilerBug,
    /// Critical vulnerability exploit or security bounty PoC (e.g. Anatoly Yakovenko / Percolator price manipulation).
    SecurityBounty,
    /// Upstream protocol contribution or ecosystem catalog (e.g. Solana Foundation, Nous Research, PEF).
    ProtocolContribution,
    /// Co-developed partner feature or integration (e.g. HolDex, CultScreener, claude-mem).
    PartnerFeature,
    /// Internal workspace architecture issue or PR (e.g. CYNIC hermeticity, CI gating).
    InternalGovernance,
}

impl InteractionCategory {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::CompilerBug => "COMPILER BUG",
            Self::SecurityBounty => "SECURITY BOUNTY",
            Self::ProtocolContribution => "PROTOCOL CONTRIBUTION",
            Self::PartnerFeature => "PARTNER FEATURE",
            Self::InternalGovernance => "INTERNAL GOVERNANCE",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            Self::CompilerBug => "badge-compiler",
            Self::SecurityBounty => "badge-bounty",
            Self::ProtocolContribution => "badge-protocol",
            Self::PartnerFeature => "badge-partner",
            Self::InternalGovernance => "badge-governance",
        }
    }
}

/// An interaction event (Pull Request or Issue) in an ecosystem or internal repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub state: String,
    pub url: String,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub is_pr: bool,
    pub is_external: bool,
    pub category: InteractionCategory,
    pub impact_note: Option<String>,
}

/// Aggregated graph of external & internal GitHub interactions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionsGraph {
    pub schema: String,
    pub total_prs: u64,
    pub external_prs: u64,
    pub internal_prs: u64,
    pub total_issues: u64,
    pub external_issues: u64,
    pub internal_issues: u64,
    pub external_ecosystems: Vec<String>,
    pub items: Vec<InteractionRecord>,
}

/// 5-tier epistemic verification status for claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    /// Recalculated and machine-proven by deterministic exit code / receipt. Zero tolerance.
    Demonstrated,
    /// Code exists and is structurally confirmed, but dynamic behavior is not replayed here.
    Implemented,
    /// Partially true, scoped, or shared attribution across multiple authors.
    Partial,
    /// Target on specification / roadmap, not yet implemented in code.
    Aspirational,
    /// Unmeasured, missing data, or intentionally undeclared.
    Unknown,
}

impl ClaimStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Demonstrated => "demonstrated",
            Self::Implemented => "implemented",
            Self::Partial => "partial",
            Self::Aspirational => "aspirational",
            Self::Unknown => "unknown",
        }
    }

    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Demonstrated => "DEMONSTRATED",
            Self::Implemented => "IMPLEMENTED",
            Self::Partial => "PARTIAL",
            Self::Aspirational => "ASPIRATIONAL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// Category of a verifiable assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimCategory {
    Architecture,
    CodeVolume,
    Hygiene,
    Provenance,
    Competence,
    Trajectory,
    Event,
}

/// Verifiable claim backed by physical machine evidence or reproducible command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiableClaim {
    pub id: String,
    pub category: ClaimCategory,
    pub statement: String,
    pub status: ClaimStatus,
    pub evidence: String,
    pub verify_command: Option<String>,
    pub numeric_value: Option<u64>,
    pub unit: Option<String>,
}

/// Chronological era of the author's work across GitHub repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoEra {
    /// Late 2023 - Early 2024: NSI high-school project, first Python scripts, algorithmic awakening.
    NsiAwakening,
    /// Late 2025: Offensive cybersecurity exploration, pentesting scripts, web dissection.
    OffensivePentest,
    /// Dec 2025 - Feb 2026: Solana memecoin trench, vanity grinding, pump.fun validators, GASdf, perps.
    SolanaTrench,
    /// Feb 2026 - Present: Low-layer sovereign Rust, CYNIC runtime, SMR, multi-agent epistemic coordination.
    SovereignRust,
}

impl RepoEra {
    pub fn label_fr(&self) -> &'static str {
        match self {
            Self::NsiAwakening => "Éveil & NSI (2023 - début 2024)",
            Self::OffensivePentest => "Déconstruction & Pentest (Fin 2025)",
            Self::SolanaTrench => "Le Feu des Tranchées Solana (Déc 2025 - Fév 2026)",
            Self::SovereignRust => "Souveraineté Rust & Multi-Agents (Fév 2026 - Présent)",
        }
    }

    pub fn label_en(&self) -> &'static str {
        match self {
            Self::NsiAwakening => "Awakening & NSI (2023 - early 2024)",
            Self::OffensivePentest => "Offensive & Pentest (Late 2025)",
            Self::SolanaTrench => "Solana Trenches & MEV (Dec 2025 - Feb 2026)",
            Self::SovereignRust => "Sovereign Rust & Multi-Agents (Feb 2026 - Present)",
        }
    }
}

/// Primary technical domain classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TechnicalDomain {
    Systems,
    Infra,
    AiAgents,
    Security,
    Blockchain,
    MarketIntel,
    DataMl,
}

impl TechnicalDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Systems => "systems",
            Self::Infra => "infra",
            Self::AiAgents => "ai_agents",
            Self::Security => "security",
            Self::Blockchain => "blockchain",
            Self::MarketIntel => "market_intel",
            Self::DataMl => "data_ml",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Systems => "Systems (Kernel / Runtime)",
            Self::Infra => "Infrastructure & DevOps",
            Self::AiAgents => "AI Agents & Orchestration",
            Self::Security => "Security & Cryptography",
            Self::Blockchain => "Blockchain (Solana / EVM)",
            Self::MarketIntel => "Market Intelligence & Quant",
            Self::DataMl => "Data & ML Pipelines",
        }
    }
}

/// Exact nature of repository authorship (zero-larp distinction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorshipNature {
    /// Original repository authored and initiated by the candidate.
    Original,
    /// Upstream fork with active, verified commits and co-development (e.g. HolDex, ASDev, claude-mem).
    ContributedFork,
    /// Upstream repository forked for security audit, architectural study, or tooling (zero author commits claimed).
    UpstreamAudit,
}

impl AuthorshipNature {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Original => "ORIGINAL",
            Self::ContributedFork => "CO-DEVELOPED FORK",
            Self::UpstreamAudit => "UPSTREAM AUDIT / FORK",
        }
    }
}

/// Provenance metadata for a GitHub / VCS repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoProvenance {
    pub name: String,
    pub description: String,
    pub is_private: bool,
    pub is_fork: bool,
    pub parent_repo: Option<String>,
    pub authorship: AuthorshipNature,
    pub author_commits: u64,
    pub created_at: String,
    pub pushed_at: String,
    pub primary_language: Option<String>,
    pub era: RepoEra,
    pub domain: TechnicalDomain,
    pub url: String,
}

/// Basis of execution: who physically writes or executes the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillBasis {
    /// Exercised directly by the human author (e.g. Python scripts, agent prompt orchestration, architecture specifications).
    Exercised,
    /// Understood conceptually and guided through AI agents, with no claimed manual typing of raw code.
    Understood,
    /// Followed, read, or surveyed without primary claim of deep mastery.
    Exposition,
}

impl SkillBasis {
    pub fn label_fr(&self) -> &'static str {
        match self {
            Self::Exercised => "exercé",
            Self::Understood => "compris",
            Self::Exposition => "exposition",
        }
    }

    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Exercised => "exercised",
            Self::Understood => "understood",
            Self::Exposition => "exposure",
        }
    }
}

/// Internal depth of conceptual grounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillDepth {
    /// Deeply anchored in fundamentals and practical debugging.
    Anchored,
    /// Actively developing and being anchored.
    InProgress,
}

impl SkillDepth {
    pub fn label_fr(&self) -> &'static str {
        match self {
            Self::Anchored => "ancré",
            Self::InProgress => "en cours",
        }
    }

    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Anchored => "anchored",
            Self::InProgress => "in progress",
        }
    }
}

/// A specific calibrated skill bullet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibratedSkillItem {
    pub text: String,
    pub basis: SkillBasis,
}

/// A grouped cluster of skills with explicit execution basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibratedSkillGroup {
    pub id: String,
    pub label_fr: String,
    pub label_en: String,
    pub basis: SkillBasis,
    pub depth: Option<SkillDepth>,
    pub domain: TechnicalDomain,
    pub note_fr: Option<String>,
    pub note_en: Option<String>,
    pub items_fr: Vec<CalibratedSkillItem>,
    pub items_en: Vec<CalibratedSkillItem>,
}

/// Identity and public profile metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorIdentity {
    /// Absent by design when the profile is pseudonymous: nothing is published unless declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    pub handle: String,
    pub github_login: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub headline_fr: String,
    pub headline_en: String,
    pub summary_fr: String,
    pub summary_en: String,
    pub disclosure_fr: String,
    pub disclosure_en: String,
    pub links: HashMap<String, String>,
}

/// Significant milestone event with proof reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalEvent {
    pub date: String,
    pub title: String,
    pub description_fr: String,
    pub description_en: String,
    /// `None` means the event is declared by the author, not backed by a checkable artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_reference: Option<String>,
}

impl AuthorIdentity {
    /// Public display name: the declared full name, else the handle.
    pub fn display_name(&self) -> &str {
        self.full_name.as_deref().unwrap_or(&self.handle)
    }
}

/// Declarative author profile (`contracts/author-profile.json`): everything the
/// ledger states about the author that cannot be measured from the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorProfile {
    pub schema: String,
    pub identity: AuthorIdentity,
    pub skills: Vec<CalibratedSkillGroup>,
    pub events: Vec<HistoricalEvent>,
}

pub const AUTHOR_PROFILE_SCHEMA_V1: &str = "cynic.author_profile.v1";

/// Sovereign Epistemic Ledger (SSOT for verifiable facts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicFactLedger {
    pub schema: String,
    pub generated_at: String,
    pub identity: AuthorIdentity,
    pub summary_metrics: HashMap<String, u64>,
    pub claims: Vec<VerifiableClaim>,
    pub repos: Vec<RepoProvenance>,
    pub skills: Vec<CalibratedSkillGroup>,
    pub events: Vec<HistoricalEvent>,
    #[serde(default)]
    pub interactions: Option<InteractionsGraph>,
}

impl EpistemicFactLedger {
    /// Converts the entire epistemic fact ledger into deterministic `DocumentChunk` records
    /// for ingestion into the BM25 lexical index and sovereign RAG retrieval.
    pub fn to_chunks(&self) -> Vec<DocumentChunk> {
        let mut chunks = Vec::new();

        // 1. Identity & Manifesto Chunk
        let id_content = format!(
            "Identity: {} (@{})\nGitHub: https://github.com/{}\nLocation: {}\n\nHeadline (FR): {}\nHeadline (EN): {}\n\nSummary (FR): {}\nSummary (EN): {}\n\nDevelopment Disclosure:\nFR: {}\nEN: {}\n",
            self.identity.display_name(),
            self.identity.handle,
            self.identity.github_login,
            self.identity.location.as_deref().unwrap_or("undisclosed"),
            self.identity.headline_fr,
            self.identity.headline_en,
            self.identity.summary_fr,
            self.identity.summary_en,
            self.identity.disclosure_fr,
            self.identity.disclosure_en
        );
        chunks.push(DocumentChunk::new(
            "epistemic://author/identity".to_string(),
            1,
            id_content.lines().count(),
            vec!["Author Epistemics".to_string(), "Identity & Thesis".to_string()],
            id_content,
            AuthorityTier::L1Specification,
        ));

        // 2. Verified Claims & Machine Proofs Chunk
        let mut claims_content = format!(
            "Verified Epistemic Claims Ledger (Schema: {})\nGenerated at: {}\n\n",
            self.schema, self.generated_at
        );
        for claim in &self.claims {
            claims_content.push_str(&format!(
                "- [{}] {}: {}\n  Evidence: {}\n  Verify: {}\n",
                claim.status.badge_label(),
                claim.id,
                claim.statement,
                claim.evidence,
                claim.verify_command.as_deref().unwrap_or("none")
            ));
            if let Some(val) = claim.numeric_value {
                claims_content.push_str(&format!(
                    "  Value: {} {}\n",
                    val,
                    claim.unit.as_deref().unwrap_or("")
                ));
            }
        }
        chunks.push(DocumentChunk::new(
            "epistemic://author/verified_claims".to_string(),
            1,
            claims_content.lines().count(),
            vec!["Author Epistemics".to_string(), "Verified Machine Receipts".to_string()],
            claims_content,
            AuthorityTier::L0Constitution,
        ));

        // 3. GitHub Multi-Repo Provenance (The 4 Eras) Chunk
        let mut repo_content = format!(
            "GitHub Repositories Provenance (Total: {} repos)\n\n",
            self.repos.len()
        );
        for era in [
            RepoEra::NsiAwakening,
            RepoEra::OffensivePentest,
            RepoEra::SolanaTrench,
            RepoEra::SovereignRust,
        ] {
            repo_content.push_str(&format!("=== {} ===\n", era.label_en()));
            for r in self.repos.iter().filter(|r| r.era == era) {
                let parent_info = if let Some(ref p) = r.parent_repo {
                    format!(" (fork of {})", p)
                } else {
                    String::new()
                };
                repo_content.push_str(&format!(
                    "- {}{} ({}, created: {}): {}\n  Domain: {} | Authorship: {} ({} commits) | Private: {} | URL: {}\n",
                    r.name,
                    parent_info,
                    r.primary_language.as_deref().unwrap_or("Unspecified"),
                    r.created_at,
                    r.description,
                    r.domain.as_str(),
                    r.authorship.badge_label(),
                    r.author_commits,
                    r.is_private,
                    r.url
                ));
            }
            repo_content.push('\n');
        }
        chunks.push(DocumentChunk::new(
            "epistemic://author/github_timeline".to_string(),
            1,
            repo_content.lines().count(),
            vec!["Author Epistemics".to_string(), "GitHub 4-Era Odyssey".to_string()],
            repo_content,
            AuthorityTier::L1Specification,
        ));

        // 4. Calibrated Skills Matrix Chunk
        let mut skills_content = "Calibrated Competencies Matrix (Three-tier Basis: Exercised / Understood / Exposure)\n\n".to_string();
        for group in &self.skills {
            skills_content.push_str(&format!(
                "## {} (Basis: {}, Depth: {})\n",
                group.label_en,
                group.basis.label_en(),
                group.depth.map(|d| d.label_en()).unwrap_or("uncalibrated")
            ));
            if let Some(note) = &group.note_en {
                skills_content.push_str(&format!("Note: {}\n", note));
            }
            for item in &group.items_en {
                skills_content.push_str(&format!("- [{}] {}\n", item.basis.label_en(), item.text));
            }
            skills_content.push('\n');
        }
        chunks.push(DocumentChunk::new(
            "epistemic://author/calibrated_skills".to_string(),
            1,
            skills_content.lines().count(),
            vec!["Author Epistemics".to_string(), "Calibrated Skills Matrix".to_string()],
            skills_content,
            AuthorityTier::L1Specification,
        ));

        // 5. External Interactions & Ecosystem Graph Chunk
        if let Some(graph) = &self.interactions {
            let mut graph_content = format!(
                "GitHub Interactions & Contribution Graph (Schema: {})\n\n",
                graph.schema
            );
            graph_content.push_str(&format!(
                "Summary:\n- Total PRs: {} (External: {}, Internal: {})\n- Total Issues: {} (External: {}, Internal: {})\n- External Ecosystems ({}): {}\n\n",
                graph.total_prs,
                graph.external_prs,
                graph.internal_prs,
                graph.total_issues,
                graph.external_issues,
                graph.internal_issues,
                graph.external_ecosystems.len(),
                graph.external_ecosystems.join(", ")
            ));

            graph_content.push_str("=== High-Impact Security Bounties & Compiler Reports ===\n");
            for item in graph.items.iter().filter(|i| {
                i.category == InteractionCategory::CompilerBug
                    || i.category == InteractionCategory::SecurityBounty
            }) {
                graph_content.push_str(&format!(
                    "- [{}] {}#{} ({}): {}\n  URL: {}\n",
                    item.category.badge_label(),
                    item.repository,
                    item.number,
                    item.state,
                    item.title,
                    item.url
                ));
                if let Some(note) = &item.impact_note {
                    graph_content.push_str(&format!("  Impact Note: {}\n", note));
                }
            }
            graph_content.push('\n');

            graph_content.push_str("=== External Protocol & Ecosystem Contributions ===\n");
            for item in graph.items.iter().filter(|i| {
                i.is_external
                    && i.category != InteractionCategory::CompilerBug
                    && i.category != InteractionCategory::SecurityBounty
            }) {
                graph_content.push_str(&format!(
                    "- [{}] {}#{} ({}): {}\n  URL: {}\n",
                    item.category.badge_label(),
                    item.repository,
                    item.number,
                    item.state,
                    item.title,
                    item.url
                ));
                if let Some(note) = &item.impact_note {
                    graph_content.push_str(&format!("  Impact Note: {}\n", note));
                }
            }

            chunks.push(DocumentChunk::new(
                "epistemic://author/interactions_graph".to_string(),
                1,
                graph_content.lines().count(),
                vec![
                    "Author Epistemics".to_string(),
                    "GitHub Interactions & Ecosystem Graph".to_string(),
                ],
                graph_content,
                AuthorityTier::L0Constitution,
            ));
        }

        chunks
    }
}
