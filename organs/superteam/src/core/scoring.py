from __future__ import annotations

from datetime import datetime, timezone

from .entities import Gate, Opportunity, OpportunityScore


CONTENT_WORDS = (
    "twitter",
    "thread",
    "video",
    "reel",
    "meme",
    "content",
    "post",
    "laugh",
    "photo",
)


TECH_WORDS = (
    "api",
    "agent",
    "skill",
    "jito",
    "geyser",
    "yellowstone",
    "transaction",
    "backend",
    "blockchain",
    "qa",
    "bug",
    "security",
    "infrastructure",
)


def score_opportunity(opportunity: Opportunity, now: datetime | None = None) -> OpportunityScore:
    now = now or datetime.now(timezone.utc)
    text = f"{opportunity.title} {opportunity.description_text}".lower()
    title = opportunity.title.lower()
    slug = opportunity.slug.lower()
    skill_text = " ".join(opportunity.skills).lower()
    days = opportunity.days_until_deadline(now)

    reasons: list[str] = []
    gates: list[Gate] = [Gate.HUMAN_SUBMISSION, Gate.EXTERNAL_BROWSER]
    score = 0.0

    reward = opportunity.reward_amount
    if reward >= 3000:
        score += 18
        reasons.append("high_reward")
    elif reward >= 1000:
        score += 10
        reasons.append("medium_reward")
    elif reward > 0:
        score += 4

    if days is not None:
        if days < 0:
            score -= 100
            reasons.append("expired_or_stale_deadline")
        elif days <= 2:
            score += 4
            reasons.append("urgent")
        elif days <= 8:
            score += 9
            reasons.append("near_term")
        else:
            score += 5

    tech_hits = sum(1 for word in TECH_WORDS if word in text or word in skill_text)
    content_hits = sum(1 for word in CONTENT_WORDS if word in title or word in slug)
    score += min(tech_hits * 5, 25)

    if tech_hits:
        reasons.append("technical_reusable_surface")
    if slug == "skills":
        score += 14
        reasons.append("directly_compounds_cynic_skills")
    if "bug" in text or "qa" in text or "security" in text:
        score += 12
        reasons.append("evidence_based_submission_possible")
    if "product feedback" in text:
        score += 8
        reasons.append("low_mutation_product_feedback")

    if content_hits:
        score -= min(content_hits * 4, 20)
        gates.append(Gate.PUBLIC_IDENTITY)
        reasons.append("public_content_or_identity_load")

    if any(word in text for word in ("wallet", "sign", "funds", "mainnet", "transfer", "trade", "stake", "deposit")):
        score -= 15
        gates.append(Gate.WALLET_OR_FUNDS)
        reasons.append("wallet_or_funds_gate")

    if any(word in text for word in ("api key", "credentials", "approved test environment", "supplied for the bounty")):
        score -= 8
        gates.append(Gate.SPONSOR_CREDENTIALS)
        reasons.append("sponsor_credentials_required")

    if slug == "skills":
        layer = "buildable_skill_submission"
        next_action = "package_public_skill_repo_or_pr"
    elif "multihopper" in slug or "bug" in title or "qa" in title:
        layer = "browser_research_then_test_credentials"
        next_action = "ask_sponsor_for_test_credentials_then_run_matrix"
    elif "transaction stack" in title or "smart-transaction-stack" in slug:
        layer = "infrastructure_sprint"
        next_action = "request_infra_credits_and_scope_minimum_lifecycle_demo"
    elif content_hits:
        layer = "human_voice_content"
        next_action = "draft_only_until_public_identity_gate"
    else:
        layer = "research_or_feedback"
        next_action = "browser_research_and_submission_outline"

    browser_task = {
        "id": f"superteam-{opportunity.slug}-evidence",
        "created_at": now.isoformat(),
        "source": "organ-superteam",
        "objective": "Collect fresh public evidence and submission requirements for this Superteam opportunity without mutating external state.",
        "url": opportunity.url,
        "context": {
            "slug": opportunity.slug,
            "title": opportunity.title,
            "deadline": opportunity.deadline,
            "reward_amount": opportunity.reward_amount,
            "reward_asset": opportunity.reward_asset,
            "resources": list(opportunity.resources),
        },
        "allowed_actions": [
            "open_public_listing",
            "open_public_resource_links",
            "extract_requirements",
            "capture_sanitized_text_evidence",
            "capture_public_screenshot_reference",
        ],
        "forbidden_actions": [
            "submit_application",
            "post_to_social",
            "connect_wallet",
            "sign_transaction",
            "trade_stake_deposit_or_transfer",
            "store_cookie_token_secret_or_private_profile_data",
        ],
        "evidence_required": [
            "current_listing_status",
            "deadline",
            "submission_requirements",
            "judging_criteria",
            "resource_links",
            "auth_or_credential_blockers",
        ],
        "human_gate": {
            "required": True,
            "reason": "External browser execution and every Superteam submission remain human-gated.",
        },
    }

    return OpportunityScore(
        slug=opportunity.slug,
        title=opportunity.title,
        score=round(score, 2),
        layer=layer,
        gates=tuple(dict.fromkeys(gates)),
        reasons=tuple(dict.fromkeys(reasons)),
        next_action=next_action,
        browser_task=browser_task,
    )
