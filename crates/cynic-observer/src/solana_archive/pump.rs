use serde::{Deserialize, Serialize};

pub const PUMP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const PUMP_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMP_FEES_PROGRAM_ID: &str = "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PumpProgramFamily {
    BondingCurve,
    Amm,
    Fees,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstructionFamily {
    Create,
    CreateV2,
    Buy,
    BuyV2,
    BuyExactQuoteIn,
    BuyExactQuoteInV2,
    Sell,
    SellV2,
    Migrate,
    CollectCreatorFee,
    CreatorFeeSharing,
    ClaimCashback,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PumpLifecycle {
    BondingCurve,
    Graduated,
    Amm,
    Unknown,
}

/// A state observation, not a complete historical truth. Optional fields are
/// deliberate: a provider or account snapshot may not expose every relation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PumpAssetState {
    pub mint: String,
    pub lifecycle: PumpLifecycle,
    pub creator: Option<String>,
    pub quote_mint: Option<String>,
    pub bonding_curve: Option<String>,
    pub amm_pool: Option<String>,
    pub creator_vault: Option<String>,
    pub sharing_config: Option<String>,
    pub virtual_quote_reserves: Option<u64>,
    pub real_quote_reserves: Option<u64>,
    pub observed_at_slot: Option<u64>,
    pub observation_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PumpProgramProfile {
    pub program_id: String,
    pub family: PumpProgramFamily,
    pub profile_version: String,
    pub decoder_source: String,
    pub known_instruction_families: Vec<InstructionFamily>,
}

#[derive(Debug, Clone, Default)]
pub struct PumpProfileRegistry {
    profiles: Vec<PumpProgramProfile>,
}

impl PumpProfileRegistry {
    pub fn canonical() -> Self {
        Self {
            profiles: vec![
                PumpProgramProfile {
                    program_id: PUMP_PROGRAM_ID.to_string(),
                    family: PumpProgramFamily::BondingCurve,
                    profile_version: "pump-public-idl-baseline".to_string(),
                    decoder_source: "versioned-idl-and-raw-fallback".to_string(),
                    known_instruction_families: vec![
                        InstructionFamily::Create,
                        InstructionFamily::CreateV2,
                        InstructionFamily::Buy,
                        InstructionFamily::BuyV2,
                        InstructionFamily::BuyExactQuoteIn,
                        InstructionFamily::BuyExactQuoteInV2,
                        InstructionFamily::Sell,
                        InstructionFamily::SellV2,
                        InstructionFamily::Migrate,
                    ],
                },
                PumpProgramProfile {
                    program_id: PUMP_AMM_PROGRAM_ID.to_string(),
                    family: PumpProgramFamily::Amm,
                    profile_version: "pump-amm-public-idl-baseline".to_string(),
                    decoder_source: "versioned-idl-and-raw-fallback".to_string(),
                    known_instruction_families: vec![
                        InstructionFamily::Buy,
                        InstructionFamily::Sell,
                        InstructionFamily::CollectCreatorFee,
                    ],
                },
                PumpProgramProfile {
                    program_id: PUMP_FEES_PROGRAM_ID.to_string(),
                    family: PumpProgramFamily::Fees,
                    profile_version: "pump-fees-public-idl-baseline".to_string(),
                    decoder_source: "versioned-idl-and-raw-fallback".to_string(),
                    known_instruction_families: vec![
                        InstructionFamily::CreatorFeeSharing,
                        InstructionFamily::ClaimCashback,
                        InstructionFamily::CollectCreatorFee,
                    ],
                },
            ],
        }
    }

    pub fn resolve(&self, program_id: &str) -> Option<&PumpProgramProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.program_id == program_id)
    }

    pub fn profiles(&self) -> &[PumpProgramProfile] {
        &self.profiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_registry_separates_program_families() {
        let registry = PumpProfileRegistry::canonical();
        assert_eq!(
            registry.resolve(PUMP_PROGRAM_ID).unwrap().family,
            PumpProgramFamily::BondingCurve
        );
        assert_eq!(
            registry.resolve(PUMP_AMM_PROGRAM_ID).unwrap().family,
            PumpProgramFamily::Amm
        );
        assert_eq!(
            registry.resolve(PUMP_FEES_PROGRAM_ID).unwrap().family,
            PumpProgramFamily::Fees
        );
    }

    #[test]
    fn unknown_program_is_not_classified_as_pump() {
        assert!(PumpProfileRegistry::canonical()
            .resolve("unknown")
            .is_none());
    }
}
