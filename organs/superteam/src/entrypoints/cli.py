from __future__ import annotations

import argparse
import dataclasses
import json
from pathlib import Path

from src.adapters.jsonl_repository import JsonlOpportunityRepository, write_json, write_jsonl
from src.core.entities import Gate, Opportunity, OpportunityScore
from src.core.scoring import score_opportunity


def main() -> int:
    parser = argparse.ArgumentParser(description="Build Superteam RAG/workbench artifacts.")
    parser.add_argument("--input", required=True, help="Path to opportunity_details.jsonl")
    parser.add_argument("--output", required=True, help="Output workbench directory")
    parser.add_argument("--limit", type=int, default=12, help="Number of top opportunities to packetize")
    args = parser.parse_args()

    input_path = Path(args.input)
    output_dir = Path(args.output)
    opportunities = JsonlOpportunityRepository(input_path).list()
    scores = sorted(
        (score_opportunity(opportunity) for opportunity in opportunities),
        key=lambda item: item.score,
        reverse=True,
    )
    by_slug = {opportunity.slug: opportunity for opportunity in opportunities}

    write_jsonl(output_dir / "rag" / "opportunity_cards.jsonl", (_rag_card(by_slug[score.slug], score) for score in scores))
    write_json(output_dir / "top_targets.json", [_score_payload(score) for score in scores[: args.limit]])
    write_jsonl(output_dir / "browser_requests.jsonl", (score.browser_task for score in scores[: args.limit]))
    write_json(output_dir / "human_gates.json", _human_gates(scores[: args.limit]))

    packet_dir = output_dir / "submission_packets"
    for score in scores[: args.limit]:
        opportunity = by_slug[score.slug]
        target = packet_dir / opportunity.slug
        write_json(target / "opportunity_score.json", _score_payload(score))
        write_json(target / "browser_task.json", score.browser_task)
        (target / "submission_draft.md").write_text(_submission_draft(opportunity, score), encoding="utf-8")

    print(json.dumps({"status": "ok", "opportunities": len(opportunities), "packetized": args.limit}, sort_keys=True))
    return 0


def _score_payload(score: OpportunityScore) -> dict:
    payload = dataclasses.asdict(score)
    payload["gates"] = [gate.value if isinstance(gate, Gate) else str(gate) for gate in score.gates]
    return payload


def _rag_card(opportunity: Opportunity, score: OpportunityScore) -> dict:
    description = " ".join(opportunity.description_text.split())
    return {
        "slug": opportunity.slug,
        "title": opportunity.title,
        "url": opportunity.url,
        "deadline": opportunity.deadline,
        "reward_amount": opportunity.reward_amount,
        "reward_asset": opportunity.reward_asset,
        "sponsor": opportunity.sponsor,
        "skills": list(opportunity.skills),
        "resources": list(dict.fromkeys(opportunity.resources)),
        "score": score.score,
        "layer": score.layer,
        "gates": [gate.value for gate in score.gates],
        "reasons": list(score.reasons),
        "retrieval_text": description[:4000],
    }


def _human_gates(scores: list[OpportunityScore]) -> dict:
    return {
        "policy": "No external submission, post, wallet connection, signature, transfer, stake, deposit, or authenticated mutation without explicit human approval.",
        "browser_runtime": "Codex did not have access to the organism browser runtime in this session; browser_requests.jsonl is the handoff queue for a browser-capable agent.",
        "targets": [
            {
                "slug": score.slug,
                "title": score.title,
                "gates": [gate.value for gate in score.gates],
                "next_action": score.next_action,
            }
            for score in scores
        ],
    }


def _submission_draft(opportunity: Opportunity, score: OpportunityScore) -> str:
    gates = ", ".join(gate.value for gate in score.gates)
    reasons = ", ".join(score.reasons)
    resources = "\n".join(f"- {item}" for item in dict.fromkeys(opportunity.resources)) or "- None detected"
    return f"""# {opportunity.title}

Superteam listing: {opportunity.url}

Status: draft only. Do not submit without human approval.

## Score

- Score: {score.score}
- Layer: {score.layer}
- Reasons: {reasons}
- Gates: {gates}
- Next action: {score.next_action}

## Public Resources To Recheck

{resources}

## Browser Evidence Needed

- Current listing status
- Deadline and prize terms
- Submission requirements
- Judging criteria
- Resource freshness
- Auth, credential, wallet, or public posting blockers

## Draft Submission Work

This packet is not a final submission. It is the workbench for collecting
browser evidence, building the artifact, and preparing a reviewer-ready payload.
"""


if __name__ == "__main__":
    raise SystemExit(main())
