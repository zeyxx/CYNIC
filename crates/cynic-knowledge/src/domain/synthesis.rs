use crate::domain::chunk::AuthorityTier;
use crate::domain::scorer::SearchHit;
use serde::{Deserialize, Serialize};

pub const SYNTHESIS_PROMPT_SCHEMA_V1: &str = "cynic.knowledge.synthesis_prompt.v1";

/// Source citée dans la synthèse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CitedSource {
    pub source_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub authority: AuthorityTier,
    pub score: f64,
}

/// Prompt structuré prêt pour l'inférence LLM avec directives anti-hallucination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisPrompt {
    pub schema: String,
    pub system_instruction: String,
    pub user_prompt: String,
    pub sources: Vec<CitedSource>,
}

pub struct SynthesisPromptBuilder;

impl SynthesisPromptBuilder {
    /// Construit le prompt borné à partir d'une requête et des chunks récupérés.
    pub fn build(query: &str, hits: &[SearchHit]) -> SynthesisPrompt {
        let mut context_buffer = String::new();
        let mut sources = Vec::new();

        for (idx, hit) in hits.iter().enumerate() {
            let ref_id = idx + 1;
            context_buffer.push_str(&format!(
                "=== [SOURCE {}] {} (Lignes {}-{}, Autorité: {:?}) ===\n{}\n\n",
                ref_id,
                hit.chunk.source_path,
                hit.chunk.line_start,
                hit.chunk.line_end,
                hit.chunk.authority,
                hit.chunk.content.trim()
            ));

            sources.push(CitedSource {
                source_path: hit.chunk.source_path.clone(),
                line_start: hit.chunk.line_start,
                line_end: hit.chunk.line_end,
                authority: hit.chunk.authority,
                score: hit.score,
            });
        }

        let system_instruction = r#"Tu es l'Oracle Épistémique de CYNIC.
Ton rôle est de répondre aux questions techniques et architecturales de l'opérateur en te basant EXCLUSIVEMENT sur les sources fournies.

DIRECTIVES IMPÉRATIVES :
1. VÉRITÉ ANCRÉE : Base toutes tes affirmations STRICTEMENT sur le texte des sources ci-dessous.
2. CITATION DES SOURCES : Cite systématiquement chaque fait ou règle avec sa référence exacte [SOURCE N: chemin:lignes].
3. PRÉCÉDENCE DE L'AUTORITÉ : Les sources L0Constitution et L1Specification prévalent toujours sur les propositions L2Design.
4. HONNÊTETÉ SUR LES LIMITES : Si une question ou un détail n'est PAS couvert par les sources, écris explicitement : "NON COUVERT PAR LE CORPUS ACTUEL", sans inventer ni extrapoler.
5. STYLE : Direct, technique, concis, en français pour les explications, zéro verbiage."#.to_string();

        let user_prompt = format!(
            "CONTEXTE DE VÉRITÉ FOURNI PAR LE SYSTÈME :\n\n{}\n\nQUESTION DE L'OPÉRATEUR :\n{}",
            context_buffer, query
        );

        SynthesisPrompt {
            schema: SYNTHESIS_PROMPT_SCHEMA_V1.to_string(),
            system_instruction,
            user_prompt,
            sources,
        }
    }

    /// Résout dynamiquement l'URL de l'endpoint d'inférence.
    /// Ordre : unité locale supervisée (fichier rendez-vous du
    /// superviseur, vérifié TCP), puis inference.toml, puis repli fixe.
    pub fn resolve_local_inference_endpoint() -> String {
        if let Ok(env_url) = std::env::var("CYNIC_INFERENCE_URL") {
            return env_url;
        }
        if let Some(live) = Self::read_supervised_endpoint() {
            return live;
        }
        let cfg_path = cynic_types::CynicPaths::inference_config_path();
        if let Ok(content) = std::fs::read_to_string(&cfg_path) {
            if let Ok(toml_val) = toml::from_str::<toml::Value>(&content) {
                if let Some(backends) = toml_val.get("backend").and_then(|b| b.as_array()) {
                    for b in backends {
                        if let Some(base_url) = b.get("base_url").and_then(|u| u.as_str()) {
                            let trimmed = base_url.trim_end_matches('/');
                            return format!("{}/chat/completions", trimmed);
                        }
                    }
                }
            }
        }
        "http://127.0.0.1:8088/v1/chat/completions".to_string()
    }

    /// Lit le rendez-vous publié par le superviseur local
    /// (`{state}/supervised-inference-endpoint` : alias puis base `/v1`).
    /// Ne retient que ce qui répond TCP, sinon rien (repli appelant).
    fn read_supervised_endpoint() -> Option<String> {
        let path = cynic_types::CynicPaths::state_dir().join("supervised-inference-endpoint");
        let raw = std::fs::read_to_string(path).ok()?;
        let mut lines = raw.lines();
        lines.next()?;
        let base = lines.next()?.trim().trim_end_matches('/');
        if base.is_empty() {
            return None;
        }
        let host_port = base
            .strip_prefix("http://")
            .or_else(|| base.strip_prefix("https://"))?;
        if Self::tcp_reachable(host_port) {
            Some(format!("{base}/chat/completions"))
        } else {
            None
        }
    }

    fn tcp_reachable(host_port: &str) -> bool {
        use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
        let addrs: Vec<SocketAddr> = host_port
            .to_socket_addrs()
            .map(|i| i.collect())
            .unwrap_or_default();
        let Some(addr) = addrs.into_iter().next() else {
            return false;
        };
        TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(800)).is_ok()
    }
}
