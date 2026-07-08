"""Domain curation rules for Hermes X tweet signals."""

import re


MIN_SIGNAL_SCORE = 4

DOMAIN_KEYWORDS = {
    "D1": [
        "rug",
        "scam",
        "pump",
        "dump",
        "liquidity",
        "supply",
        "holders",
        "bundled",
        "rug pull",
        "slow rug",
        "fee assignment",
    ],
    "D2": [
        "inference",
        "llm",
        "vllm",
        "sglang",
        "quantization",
        "gguf",
        "tok/s",
        "throughput",
        "latency",
        "model",
        "gpu",
        "cuda",
        "kv cache",
        "attention",
        "tph",
        "tps",
        "agent",
        "thinking",
    ],
    "D3": [
        "self custody",
        "self-custody",
        "sovereign",
        "censorship resist",
        "decentraliz",
        "permissionless",
        "trustless",
        "own your keys",
        "metadao",
        "futarchy",
        "futardio",
        "prediction market",
        "polymarket",
        "kalshi",
        "forecast market",
        "p2p wager",
        "sports bet",
        "sportsfi",
        "chess solana",
        "talaria",
        "skill game",
        "player vs player",
    ],
    "D4": [
        "security",
        "exploit",
        "vulnerability",
        "hack",
        "phishing",
        "malware",
        "warning",
        "scam",
        "fraud",
        "bridge",
        "cross-chain",
    ],
    "D5": [
        "market",
        "price",
        "bull",
        "bear",
        "trend",
        "volatility",
        "cycle",
        "adoption",
        "ecosystem",
        "regulation",
        "gmt",
        "sol",
        "eth",
        "btc",
    ],
    "D6": [
        "proof",
        "evidence",
        "verified",
        "confirmed",
        "predicted",
        "forecast",
        "accuracy",
        "epistemic",
        "truth",
        "certainty",
        "error",
    ],
}

DEFAULT_NARRATIVE_MAPPINGS = {
    "D1": ["rug_warning", "token_launch", "meme_token", "pump_hype", "solana_token"],
    "D2": ["inference", "llm", "agent", "open_weights", "ai_crypto"],
    "D3": ["self_custody", "sovereignty", "decentralization"],
    "D4": ["rug_pull", "honeypot", "exploit", "scam", "vulnerability"],
    "D5": ["macro", "institutional", "market", "regulation"],
    "D6": ["epistemology", "bounded_judgment"],
}


def extract_keywords(text: str) -> list[str]:
    """Extract meaningful keywords from tweet text."""
    if not text:
        return []

    text_lower = text.lower()
    text_clean = re.sub(r"https?://\S+", "", text_lower)
    text_clean = re.sub(r"@\w+", "", text_clean)
    text_clean = re.sub(r"#\w+", "", text_clean)
    text_clean = re.sub(r"\$[a-z]{2,10}", "", text_clean)
    words = re.findall(r"\b[a-z]{3,}\b", text_clean)

    stopwords = {
        "the",
        "and",
        "for",
        "that",
        "this",
        "with",
        "from",
        "are",
        "but",
        "not",
        "have",
        "been",
        "will",
        "can",
        "just",
        "like",
        "one",
        "new",
        "who",
        "what",
        "when",
        "where",
        "why",
        "how",
        "all",
        "some",
        "any",
        "out",
        "get",
        "has",
        "his",
        "her",
        "its",
        "our",
        "your",
        "make",
        "day",
        "time",
        "way",
        "year",
        "work",
        "people",
        "did",
        "don",
        "want",
        "know",
        "said",
        "say",
        "use",
        "find",
        "give",
        "tell",
        "come",
    }

    keywords = [word for word in words if word not in stopwords and len(word) > 2]
    return list(set(keywords[:10]))


def assign_domain(
    text: str,
    narratives: list[str],
    tier: str,
    narrative_mappings: dict[str, list[str]] | None = None,
) -> str:
    """Assign tweet to domain D1-D6."""
    mappings = narrative_mappings or DEFAULT_NARRATIVE_MAPPINGS
    text_lower = text.lower()

    if narratives:
        for domain_id, domain_narratives in mappings.items():
            if any(narrative in narratives for narrative in domain_narratives):
                return domain_id

    for domain, keywords in DOMAIN_KEYWORDS.items():
        if any(keyword in text_lower for keyword in keywords):
            return domain

    if tier == "curated":
        return "D6"
    if tier in ["whale", "influencer"]:
        return "D5"

    return "D1"


def extract_falsifiable_claim(text: str, domain: str) -> str:
    """Extract or generate a falsifiable claim from tweet text."""
    text_lower = text.lower()

    if "if" in text_lower and ("then" in text_lower or ":" in text):
        match = re.search(r"if[^.!?]*[.:?!]", text, re.IGNORECASE | re.DOTALL)
        if match:
            return match.group(0)[:200]

    if domain == "D1":
        if any(keyword in text_lower for keyword in ["rug pull", "slow rug", "honeypot"]):
            return "If pattern manifests: token will lose >80% value within 6mo"
        if "liquidity" in text_lower and ("locked" in text_lower or "permanently" in text_lower):
            return "If verified: liquidity mechanics prevent early withdrawal"
        return "If verified: token mechanism claim is accurate"

    if domain == "D2":
        has_digit = any(digit in text for digit in "0123456789")
        if any(keyword in text_lower for keyword in ["tok/s", "tokens/sec", "throughput"]) and has_digit:
            return "If benchmarked: claimed throughput achievable under stated conditions"
        if any(keyword in text_lower for keyword in ["latency", "inference time", "response time"]):
            return "If measured: latency within specified bounds"
        return "If verified: performance claim is testable"

    if domain == "D3":
        if any(keyword in text_lower for keyword in ["prediction market", "polymarket", "kalshi", "forecast market"]):
            return "If measured: prediction market resolves at stated probability within stated timeframe"
        if any(keyword in text_lower for keyword in ["metadao", "futarchy", "futardio"]):
            return "If verified: futarchy governance outcome matches market-forecasted decision"
        if any(keyword in text_lower for keyword in ["p2p wager", "player vs player", "chess", "skill game"]):
            return "If tested: P2P wager mechanism settles correctly on verified skill outcome"
        if any(keyword in text_lower for keyword in ["self custody", "self-custody", "own your keys"]):
            return "If verified: self-custody mechanism works as described"
        return "If verified: sovereignty claim holds under examination"

    if domain == "D4":
        if any(keyword in text_lower for keyword in ["vulnerability", "exploit", "cve", "hack", "breach"]):
            return "If tested: vulnerability is reproducible and exploitable"
        return "If verified: security claim can be validated"

    if domain == "D5":
        if any(keyword in text_lower for keyword in ["tps", "tvl", "volume", "adoption", "growth"]):
            return "If measured: metric value matches on-chain data"
        return "If verified: market claim aligns with observable data"

    if domain == "D6":
        if any(keyword in text_lower for keyword in ["predicted", "forecast", "accuracy"]):
            return "If verified: prediction accuracy > random baseline"
        if any(keyword in text_lower for keyword in ["proof", "evidence", "verified"]):
            return "If tested: claim withstands critical examination"
        return "If verified: claim's truth value is determinable"

    first_sentence = text.split(".")[0].strip()
    if 20 < len(first_sentence) < 150:
        return f"If verified: {first_sentence}"
    return "If verified: claim content is accurate"


def create_signal_id(author: str, tweet_id: str, domain: str, idx: int) -> str:
    """Create a unique signal ID."""
    return f"{domain}_{author}_{tweet_id}_{idx:04d}"


def signal_strength(raw_score: int | float) -> float:
    """Scale tweet score into the K15-compatible signal strength range."""
    return min(0.95, (raw_score - MIN_SIGNAL_SCORE) / 3 * 0.5 + 0.4)


SKIP_AUTHORS = {
    "gary_recovery_",
    "thelipglossguy_",
    "assertguard",
    "ep_peter",
    "arthurbomb",
    "emcruzzz",
    "jeffersonyoan",
    "njabulobhabha",
}


def domain_signal(
    tweet: dict,
    domain: str,
    idx: int,
) -> dict:
    """Build a DomainSignal payload from a high-signal tweet."""
    author = tweet.get("author_screen_name", "unknown")
    text = tweet.get("text", "")
    tweet_id = tweet.get("tweet_id", "")
    tier = tweet.get("author_tier", "unknown")
    narratives = tweet.get("narratives", [])

    return {
        "signal_id": create_signal_id(author, tweet_id, domain, idx),
        "domain": domain,
        "pattern": text[:250],
        "strength": signal_strength(tweet.get("signal_score", 0)),
        "sources": [tweet_id, author],
        "falsifiable_claim": extract_falsifiable_claim(text, domain),
        "narratives": narratives,
        "author_tier": tier,
    }


def curate_by_domain(
    tweets: list[dict],
    narrative_mappings: dict[str, list[str]] | None = None,
) -> dict[str, list[dict]]:
    """Group high-signal tweets into DomainSignal collections."""
    by_domain: dict[str, list[dict]] = {}
    signal_counter: dict[str, int] = {}

    for tweet in tweets:
        if tweet.get("author_screen_name", "").lower() in SKIP_AUTHORS:
            continue

        domain = assign_domain(
            tweet.get("text", ""),
            tweet.get("narratives", []),
            tweet.get("author_tier", "unknown"),
            narrative_mappings,
        )
        idx = signal_counter.get(domain, 0)
        signal_counter[domain] = idx + 1
        by_domain.setdefault(domain, []).append(domain_signal(tweet, domain, idx))

    return by_domain
