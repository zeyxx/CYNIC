use super::{InstructionFamily, PumpProfileRegistry, PumpProgramFamily};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstructionDecodeStatus {
    Classified,
    ProgramKnownInstructionUnknown,
    ProgramUnknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub path: String,
    pub program_id: Option<String>,
    pub program_family: PumpProgramFamily,
    pub profile_version: Option<String>,
    pub instruction_name: Option<String>,
    pub instruction_family: InstructionFamily,
    pub status: InstructionDecodeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecodedTransaction {
    pub signature: String,
    pub slot: Option<u64>,
    pub instructions: Vec<DecodedInstruction>,
    pub unresolved_paths: Vec<String>,
}

pub fn classify_instruction(
    registry: &PumpProfileRegistry,
    program_id: Option<&str>,
    instruction_name: Option<&str>,
    path: impl Into<String>,
) -> DecodedInstruction {
    let path = path.into();
    let Some(program_id) = program_id else {
        return DecodedInstruction {
            path,
            program_id: None,
            program_family: PumpProgramFamily::Unknown,
            profile_version: None,
            instruction_name: instruction_name.map(ToOwned::to_owned),
            instruction_family: InstructionFamily::Unknown,
            status: InstructionDecodeStatus::ProgramUnknown,
        };
    };

    let Some(profile) = registry.resolve(program_id) else {
        return DecodedInstruction {
            path,
            program_id: Some(program_id.to_string()),
            program_family: PumpProgramFamily::Unknown,
            profile_version: None,
            instruction_name: instruction_name.map(ToOwned::to_owned),
            instruction_family: InstructionFamily::Unknown,
            status: InstructionDecodeStatus::ProgramUnknown,
        };
    };

    let family = instruction_name.and_then(instruction_family);
    let status = if family.is_some() {
        InstructionDecodeStatus::Classified
    } else {
        InstructionDecodeStatus::ProgramKnownInstructionUnknown
    };
    DecodedInstruction {
        path,
        program_id: Some(program_id.to_string()),
        program_family: profile.family.clone(),
        profile_version: Some(profile.profile_version.clone()),
        instruction_name: instruction_name.map(ToOwned::to_owned),
        instruction_family: family.unwrap_or(InstructionFamily::Unknown),
        status,
    }
}

pub fn decode_transaction(registry: &PumpProfileRegistry, value: &Value) -> DecodedTransaction {
    let signature = value
        .get("observation")
        .and_then(|observation| observation.get("signature"))
        .and_then(Value::as_str)
        .unwrap_or("unknown-signature")
        .to_string();
    let slot = value
        .get("observation")
        .and_then(|observation| observation.get("slot"))
        .and_then(Value::as_u64);
    let message = value
        .get("transaction")
        .and_then(|transaction| transaction.get("transaction"))
        .and_then(|transaction| transaction.get("message"));
    let account_keys = message
        .and_then(|message| message.get("accountKeys"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut instructions = Vec::new();
    let mut unresolved_paths = Vec::new();

    if let Some(top_level) = message
        .and_then(|message| message.get("instructions"))
        .and_then(Value::as_array)
    {
        for (index, instruction) in top_level.iter().enumerate() {
            let path = format!("message.instructions[{index}]");
            let decoded = decode_value_instruction(registry, instruction, &account_keys, path);
            if decoded.status != InstructionDecodeStatus::Classified {
                unresolved_paths.push(decoded.path.clone());
            }
            instructions.push(decoded);
        }
    }

    if let Some(inner_groups) = value
        .get("transaction")
        .and_then(|transaction| transaction.get("meta"))
        .and_then(|meta| meta.get("innerInstructions"))
        .and_then(Value::as_array)
    {
        for group in inner_groups {
            let parent_index = group
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            if let Some(inner) = group.get("instructions").and_then(Value::as_array) {
                for (index, instruction) in inner.iter().enumerate() {
                    let path = format!("meta.innerInstructions[{parent_index}][{index}]");
                    let decoded =
                        decode_value_instruction(registry, instruction, &account_keys, path);
                    if decoded.status != InstructionDecodeStatus::Classified {
                        unresolved_paths.push(decoded.path.clone());
                    }
                    instructions.push(decoded);
                }
            }
        }
    }

    DecodedTransaction {
        signature,
        slot,
        instructions,
        unresolved_paths,
    }
}

fn decode_value_instruction(
    registry: &PumpProfileRegistry,
    instruction: &Value,
    account_keys: &[Value],
    path: String,
) -> DecodedInstruction {
    let program_id = instruction
        .get("programId")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            instruction
                .get("programIdIndex")
                .and_then(Value::as_u64)
                .and_then(|index| account_keys.get(index as usize))
                .and_then(account_key)
        });
    let instruction_name = instruction
        .get("parsed")
        .and_then(|parsed| parsed.get("type"))
        .and_then(Value::as_str)
        .or_else(|| instruction.get("name").and_then(Value::as_str));
    classify_instruction(registry, program_id.as_deref(), instruction_name, path)
}

fn account_key(value: &Value) -> Option<String> {
    value.as_str().map(ToOwned::to_owned).or_else(|| {
        value
            .get("pubkey")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn instruction_family(name: &str) -> Option<InstructionFamily> {
    let normalized = name.to_ascii_lowercase().replace('-', "_");
    Some(match normalized.as_str() {
        "create" => InstructionFamily::Create,
        "create_v2" => InstructionFamily::CreateV2,
        "buy" => InstructionFamily::Buy,
        "buy_v2" => InstructionFamily::BuyV2,
        "buy_exact_quote_in" => InstructionFamily::BuyExactQuoteIn,
        "buy_exact_quote_in_v2" => InstructionFamily::BuyExactQuoteInV2,
        "sell" => InstructionFamily::Sell,
        "sell_v2" => InstructionFamily::SellV2,
        "migrate" => InstructionFamily::Migrate,
        "collect_creator_fee" => InstructionFamily::CollectCreatorFee,
        "creator_fee_sharing" => InstructionFamily::CreatorFeeSharing,
        "claim_cashback" => InstructionFamily::ClaimCashback,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::PUMP_PROGRAM_ID;
    use serde_json::json;

    #[test]
    fn known_program_and_v2_instruction_are_classified() {
        let decoded = classify_instruction(
            &PumpProfileRegistry::canonical(),
            Some(PUMP_PROGRAM_ID),
            Some("buy_v2"),
            "message.instructions[0]",
        );
        assert_eq!(decoded.instruction_family, InstructionFamily::BuyV2);
        assert_eq!(decoded.status, InstructionDecodeStatus::Classified);
    }

    #[test]
    fn unknown_instruction_is_preserved_as_unknown() {
        let decoded = classify_instruction(
            &PumpProfileRegistry::canonical(),
            Some(PUMP_PROGRAM_ID),
            Some("future_instruction"),
            "message.instructions[0]",
        );
        assert_eq!(
            decoded.status,
            InstructionDecodeStatus::ProgramKnownInstructionUnknown
        );
    }

    #[test]
    fn transaction_decoder_reads_top_level_and_inner_instructions() {
        let value = json!({
            "observation": {"signature": "sig", "slot": 10},
            "transaction": {
                "transaction": {
                    "message": {
                        "accountKeys": [{"pubkey": PUMP_PROGRAM_ID}],
                        "instructions": [{"programIdIndex": 0, "parsed": {"type": "buy_v2"}}]
                    }
                },
                "meta": {
                    "innerInstructions": [{"index": 0, "instructions": [
                        {"programId": PUMP_PROGRAM_ID, "parsed": {"type": "sell"}}
                    ]}]
                }
            }
        });
        let decoded = decode_transaction(&PumpProfileRegistry::canonical(), &value);
        assert_eq!(decoded.instructions.len(), 2);
        assert!(decoded.unresolved_paths.is_empty());
    }
}
