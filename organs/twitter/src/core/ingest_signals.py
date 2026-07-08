"""Pure signal-shaping rules for Twitter/X ingestion.

This module owns deterministic domain assignment and Dog stimulus formatting.
It has no filesystem, network, kernel, or browser side effects.
"""


def assign_domain(tweet: dict, domains: dict, narrative_mappings: dict) -> str:
    """Assign domain to tweet: narrative-first, then keyword, then default D1."""
    narratives = tweet.get("narratives", []) or []
    for narrative in narratives:
        for domain_id, narrative_list in narrative_mappings.items():
            if narrative in narrative_list:
                return domain_id

    text = tweet.get("text", "").lower()
    for domain_id, domain_info in domains.items():
        if not isinstance(domain_info, dict):
            continue
        keywords = domain_info.get("keywords", [])
        if any(kw.lower() in text for kw in keywords):
            return domain_id

    return "D1"


def build_social_stimulus(row: dict, domain: str) -> str:
    """Build structured stimulus from tweet data for Dog evaluation."""
    text = row.get("text", "")[:500]
    author = row.get("author_screen_name", "?")
    score = row.get("signal_score", 0)
    tier = row.get("author_tier", "unknown")
    cashtags = row.get("cashtags", [])
    narratives = row.get("narratives", [])
    views = row.get("views", 0) or 0
    likes = row.get("likes", 0) or 0
    replies = row.get("replies", 0) or 0
    bookmarks = row.get("bookmarks", 0) or 0
    followers = row.get("author_followers_count", 0) or 0
    engagement = row.get("engagement_rate", 0.0) or 0.0
    interaction = row.get("interaction_type", "original")
    is_coordinated = row.get("is_coordinated", False)
    coord_count = row.get("coordination_count", 0)

    lines = []
    lines.append(f"[DOMAIN: {domain}]")
    lines.append("")
    lines.append("[METRICS]")
    lines.append(f"author: @{author}")
    lines.append(f"author_tier: {tier}")
    lines.append(f"author_followers: {followers:,}")
    lines.append(f"verified: {row.get('author_verified', False)}")
    lines.append(f"signal_score: {score} (heuristic, range -5 to +7)")
    lines.append(f"interaction_type: {interaction}")
    lines.append(f"views: {views:,}")
    lines.append(f"likes: {likes:,}")
    lines.append(f"replies: {replies:,}")
    lines.append(f"bookmarks: {bookmarks:,}")
    lines.append(f"engagement_rate: {engagement:.4f}")
    if cashtags:
        lines.append(f"cashtags: {', '.join(cashtags)}")
    if narratives:
        lines.append(f"narratives: {', '.join(narratives)}")
    if is_coordinated:
        lines.append(f"COORDINATION_FLAG: {coord_count} identical texts detected")
    lines.append(f"lang: {row.get('lang', '?')}")
    lines.append("")
    lines.append("[CONTENT]")
    lines.append(text)
    lines.append("")
    lines.append("[BASELINES]")
    lines.append("high_signal: signal_score>=5, engagement>0.02, author_tier=whale/influencer, original content, narratives=analysis/warning")
    lines.append("moderate_signal: signal_score 3-4, engagement 0.005-0.02, verified author, some discussion")
    lines.append("noise: signal_score<3, engagement<0.005, bot tier, retweet, no cashtags/narratives")
    lines.append("coordination_risk: coordination_count>=3, identical text from multiple authors")
    lines.append("")
    lines.append("[AXIOM EVIDENCE]")
    lines.append("FIDELITY: Is this author's claim genuine? Does the content match observable on-chain reality?")
    lines.append("PHI: Is engagement proportional to content quality? Is follower-to-engagement ratio coherent?")
    lines.append("VERIFY: Can the claims in this tweet be verified? Are cashtags/narratives checkable on-chain?")
    lines.append("CULTURE: Does this follow established crypto discourse norms? Quality thread vs pump spam?")
    lines.append("BURN: Is the signal efficient? Original content vs noise (retweet, bot, coordinated)?")
    lines.append("SOVEREIGNTY: Is this independent analysis or coordinated shilling? Author tier vs content quality?")
    lines.append("")
    lines.append("[QUESTION]")
    lines.append("Evaluate this social signal's reliability and information value. Score each axiom from 0.05 to 0.618.")

    return "\n".join(lines)
