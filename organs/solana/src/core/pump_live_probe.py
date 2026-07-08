"""Live Pump/PumpSwap chain evidence classification."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


PUMP_PROGRAM_ID = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
PUMPSWAP_PROGRAM_ID = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"
SPL_TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
TOKEN_2022_PROGRAM_ID = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"

TOKEN_PROGRAMS = {
    SPL_TOKEN_PROGRAM_ID: "spl-token",
    TOKEN_2022_PROGRAM_ID: "token-2022",
}


@dataclass(frozen=True)
class MintProbe:
    mint: str
    account: dict[str, Any] | None
    signatures: list[dict[str, Any]]
    transactions: list[dict[str, Any]]


def classify_mint_probe(probe: MintProbe) -> dict[str, Any]:
    account_result = (probe.account or {}).get("result")
    account_value = account_result.get("value") if isinstance(account_result, dict) else None
    owner = account_value.get("owner") if isinstance(account_value, dict) else None
    parsed_info = _parsed_mint_info(account_value)
    seen_programs = sorted(_program_ids_seen(probe.transactions))
    evidence = {
        "mint_suffix_pump": probe.mint.endswith("pump"),
        "account_exists": account_value is not None,
        "mint_owner_program": owner,
        "token_program_kind": TOKEN_PROGRAMS.get(owner, "unknown" if owner else None),
        "pump_program_seen_in_sample": PUMP_PROGRAM_ID in seen_programs,
        "pumpswap_program_seen_in_sample": PUMPSWAP_PROGRAM_ID in seen_programs,
        "sampled_signature_count": len(probe.signatures),
        "sampled_transaction_count": len(probe.transactions),
        "seen_programs": seen_programs,
    }

    classification = "unknown"
    if evidence["pump_program_seen_in_sample"] and evidence["pumpswap_program_seen_in_sample"]:
        classification = "pump_origin_with_pumpswap_evidence"
    elif evidence["pump_program_seen_in_sample"]:
        classification = "pump_origin_evidence"
    elif evidence["pumpswap_program_seen_in_sample"]:
        classification = "pumpswap_evidence_without_sampled_pump_origin"
    elif evidence["mint_suffix_pump"]:
        classification = "pump_suffix_only"

    return {
        "schema": "cynic.solana.pump_live_probe.v1",
        "domain": "solana",
        "kind": "mechanism_observation",
        "target": f"solana:mint:{probe.mint}",
        "mint": probe.mint,
        "classification": classification,
        "confidence": _confidence(classification, evidence),
        "mint_info": parsed_info,
        "evidence": evidence,
        "next_verification": _next_verification(classification, evidence),
    }


def _parsed_mint_info(account_value: dict[str, Any] | None) -> dict[str, Any] | None:
    if not isinstance(account_value, dict):
        return None
    data = account_value.get("data")
    if not isinstance(data, dict):
        return None
    parsed = data.get("parsed")
    if not isinstance(parsed, dict):
        return None
    info = parsed.get("info")
    if not isinstance(info, dict):
        return None
    return {
        "decimals": info.get("decimals"),
        "supply": info.get("supply"),
        "is_initialized": info.get("isInitialized"),
        "mint_authority": info.get("mintAuthority"),
        "freeze_authority": info.get("freezeAuthority"),
    }


def _program_ids_seen(transactions: list[dict[str, Any]]) -> set[str]:
    programs: set[str] = set()
    for tx in transactions:
        result = tx.get("result") if isinstance(tx, dict) else None
        if not isinstance(result, dict):
            continue
        transaction = result.get("transaction")
        if isinstance(transaction, dict):
            message = transaction.get("message")
            if isinstance(message, dict):
                _collect_instruction_programs(message.get("instructions"), programs)
        meta = result.get("meta")
        if isinstance(meta, dict):
            for group in meta.get("innerInstructions", []):
                if isinstance(group, dict):
                    _collect_instruction_programs(group.get("instructions"), programs)
    return programs


def _collect_instruction_programs(instructions: Any, programs: set[str]) -> None:
    if not isinstance(instructions, list):
        return
    for instruction in instructions:
        if not isinstance(instruction, dict):
            continue
        program_id = instruction.get("programId")
        if isinstance(program_id, str):
            programs.add(program_id)


def _confidence(classification: str, evidence: dict[str, Any]) -> str:
    if classification == "pump_origin_with_pumpswap_evidence":
        return "observed"
    if classification in {"pump_origin_evidence", "pumpswap_evidence_without_sampled_pump_origin"}:
        return "observed_sample"
    if evidence.get("mint_suffix_pump"):
        return "weak_heuristic"
    return "unknown"


def _next_verification(classification: str, evidence: dict[str, Any]) -> list[str]:
    steps: list[str] = []
    if not evidence.get("account_exists"):
        return ["verify mint address and RPC endpoint"]
    if classification != "pump_origin_with_pumpswap_evidence":
        steps.append("increase transaction sample and search create/migrate instructions")
    if not evidence.get("pumpswap_program_seen_in_sample"):
        steps.append("verify whether the mint has a PumpSwap pool")
    if evidence.get("token_program_kind") == "token-2022":
        steps.append("inspect Token-2022 extensions before contract design")
    return steps
