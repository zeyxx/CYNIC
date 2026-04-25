/// Wallet enrichment utilities for wallet-judgment domain.
///
/// B&C (Blitz & Chill) backend uses this to construct the enriched_context
/// for wallet authenticity evaluation. T. provides this as a reusable library
/// so B&C doesn't need to reimplement wallet-judgment signal extraction.
///
/// Output: formatted markdown string suitable for stimulus.context.
use crate::domain::dog::PHI_INV4;

/// Game data from B&C's LocalCompletedGame store (client-provided).
#[derive(Debug, Clone)]
pub struct LocalGameRecord {
    /// ISO 8601 timestamp of game completion
    pub timestamp: String,
    /// Game duration in seconds
    pub duration_seconds: u32,
    /// Archetype verdict (e.g., "The Aggressive", "The Pragmatist")
    pub archetype: String,
    /// Hash of the game's move sequence (for uniqueness detection)
    pub move_hash: String,
    /// List of opening sequences played (as descriptive strings)
    pub openings: Vec<String>,
}

/// Wallet enrichment context — builder for stimulus enrichment.
#[derive(Debug)]
pub struct WalletEnrichmentBuilder {
    wallet_address: String,
    games: Vec<LocalGameRecord>,
    wallet_age_days: u32,
    // Flags detected by B&C (not computed here)
    suspicious_cluster: bool,
    replay_risk: bool,
}

impl WalletEnrichmentBuilder {
    /// Create a new wallet enrichment builder.
    pub fn new(wallet_address: String, wallet_age_days: u32) -> Self {
        Self {
            wallet_address,
            games: Vec::new(),
            wallet_age_days,
            suspicious_cluster: false,
            replay_risk: false,
        }
    }

    /// Add a game record.
    pub fn add_game(mut self, game: LocalGameRecord) -> Self {
        self.games.push(game);
        self
    }

    /// Add multiple game records.
    pub fn with_games(mut self, games: Vec<LocalGameRecord>) -> Self {
        self.games = games;
        self
    }

    /// Set suspicious clustering flag (coordinated activity detected).
    pub fn set_suspicious_cluster(mut self, flag: bool) -> Self {
        self.suspicious_cluster = flag;
        self
    }

    /// Set replay risk flag (moves match another wallet's game).
    pub fn set_replay_risk(mut self, flag: bool) -> Self {
        self.replay_risk = flag;
        self
    }

    /// Build the enriched context string for stimulus injection.
    pub fn build(self) -> String {
        let games_completed = self.games.len();

        // Compute archetype consistency (% of games in modal archetype)
        let archetype_consistency = if games_completed > 0 {
            let mut archetype_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for game in &self.games {
                *archetype_counts.entry(game.archetype.clone()).or_insert(0) += 1;
            }
            let max_count = *archetype_counts.values().max().unwrap_or(&0);
            ((max_count as f64) / (games_completed as f64)) * 100.0
        } else {
            0.0
        };

        // Compute duration variance (coefficient of variation)
        let (average_duration, duration_variance) = if games_completed > 0 {
            let sum: u64 = self.games.iter().map(|g| g.duration_seconds as u64).sum();
            let mean = (sum as f64) / (games_completed as f64);

            let variance: f64 = self
                .games
                .iter()
                .map(|g| {
                    let diff = (g.duration_seconds as f64) - mean;
                    diff * diff
                })
                .sum::<f64>()
                / (games_completed as f64);
            let std_dev = variance.sqrt();
            let cv = if mean > 0.0 { std_dev / mean } else { 0.0 };

            (mean, cv)
        } else {
            (0.0, 0.0)
        };

        // Extract archetype sequence
        let archetype_sequence: Vec<String> =
            self.games.iter().map(|g| g.archetype.clone()).collect();

        // Extract timestamps
        let game_timestamps: Vec<String> = self.games.iter().map(|g| g.timestamp.clone()).collect();

        // Compute opening repertoire (count unique openings, hash)
        let all_openings: Vec<String> = self
            .games
            .iter()
            .flat_map(|g| g.openings.iter().cloned())
            .collect();
        let unique_openings: std::collections::HashSet<String> =
            all_openings.iter().cloned().collect();
        let opening_repertoire_hash =
            format!("0x{:x}", std::collections::hash_map::DefaultHasher::new()); // Placeholder; real implementation would hash

        // Opening frequency (top 3)
        let mut opening_freq: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for opening in all_openings {
            *opening_freq.entry(opening).or_insert(0) += 1;
        }
        let mut top_openings: Vec<_> = opening_freq.into_iter().collect();
        top_openings.sort_by(|a, b| b.1.cmp(&a.1));
        let top_3: Vec<String> = top_openings
            .iter()
            .take(3)
            .map(|(o, c)| {
                let pct = if unique_openings.len() > 0 {
                    ((c * 100) / unique_openings.len())
                } else {
                    0
                };
                format!("{} ({}%)", o, pct)
            })
            .collect();

        // Move sequence hashes (placeholder)
        let move_sequence_hash =
            format!("0x{:x}", std::collections::hash_map::DefaultHasher::new()); // Placeholder

        // Build formatted context
        format!(
            r#"WALLET PROFILE
Address: {}
Games completed: {}
Wallet age: {} days
Archetype consistency: {:.1}%

GAME HISTORY
Archetypes: [{}]
Game timestamps: [{}]
Game durations (seconds): [{}]
Average duration: {:.1}s
Duration variance (σ/μ): {:.2}

OPENING REPERTOIRE
Hash: {}
Unique openings: {}
Top 3: {}

MOVE SEQUENCE ANALYSIS
Move hash: {}
Replay risk: {}
Coordination signals: {}

FLAGS
Suspicious clustering: {}
Age < 5 days: {}
Duration variance > 0.50: {}
Variance > φ⁻⁴ (0.146): {}
"#,
            self.wallet_address,
            games_completed,
            self.wallet_age_days,
            archetype_consistency,
            archetype_sequence
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", "),
            game_timestamps
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", "),
            self.games
                .iter()
                .map(|g| g.duration_seconds.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            average_duration,
            duration_variance,
            opening_repertoire_hash,
            unique_openings.len(),
            top_3.join(", "),
            move_sequence_hash,
            self.replay_risk,
            if self.suspicious_cluster {
                "detected"
            } else {
                "none"
            },
            self.suspicious_cluster,
            self.wallet_age_days < 5,
            duration_variance > 0.50,
            duration_variance > PHI_INV4,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_enrichment_basic() {
        let builder = WalletEnrichmentBuilder::new("DemoWallet123".to_string(), 24);
        let game1 = LocalGameRecord {
            timestamp: "2026-04-10T14:32:00Z".to_string(),
            duration_seconds: 342,
            archetype: "The Aggressive".to_string(),
            move_hash: "0x1234".to_string(),
            openings: vec!["Italian Game".to_string(), "King's Indian".to_string()],
        };
        let game2 = LocalGameRecord {
            timestamp: "2026-04-11T15:45:00Z".to_string(),
            duration_seconds: 287,
            archetype: "The Aggressive".to_string(),
            move_hash: "0x5678".to_string(),
            openings: vec!["Italian Game".to_string()],
        };

        let context = builder
            .with_games(vec![game1, game2])
            .set_suspicious_cluster(false)
            .set_replay_risk(false)
            .build();

        assert!(context.contains("Games completed: 2"));
        assert!(context.contains("Wallet age: 24 days"));
        assert!(context.contains("Archetype consistency: 100.0%"));
        assert!(context.contains("Suspicious clustering: false"));
        assert!(context.contains("Replay risk: false"));
    }
}
