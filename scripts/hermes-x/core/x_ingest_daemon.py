"""
CYNIC Ingest Daemon — tails a JSONL dataset and POSTs observations to the kernel.

Universal: works for any organ that writes enriched JSONL.
Seeks to end of file on start, then polls for new lines.

Usage:
    python x_ingest_daemon.py                          # defaults: hermes/x dataset
    python x_ingest_daemon.py --dataset /path/to.jsonl # custom dataset
    python x_ingest_daemon.py --replay                 # replay full file (one-shot)

Environment:
    CYNIC_REST_ADDR  — kernel address
    CYNIC_API_KEY    — kernel auth token
    X_DATASET_PATH   — default dataset path (set by systemd)
"""

import argparse
import json
import os
import logging
import signal
import sys
import time
from pathlib import Path
from datetime import datetime

import requests
import yaml

from hermes_paths import DATASET as DEFAULT_DATASET, HERMES_X_DIR as DEFAULT_ORGAN_DIR
from get_x_credentials import get_x_credentials

logger = logging.getLogger("x-ingest")
ACCOUNT_ID = os.environ.get("HERMES_ACCOUNT", "cynic")
KERNEL_ADDR = ""
API_KEY = ""
ORGAN_NAME = f"x-proxy-{ACCOUNT_ID}"
DOMAIN = "twitter"
POLL_INTERVAL = 2.0
BATCH_SIZE = 20
HEARTBEAT_INTERVAL = 60.0  # seconds between heartbeat /observe posts
JUDGE_THROTTLE = 2.0  # seconds between /judge calls (Dogs need inference time)


def load_env():
    global KERNEL_ADDR, API_KEY
    KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = os.environ.get("CYNIC_API_KEY", "")
    if KERNEL_ADDR and API_KEY:
        return
    env_file = Path.home() / ".cynic-env"
    if not env_file.exists():
        return
    for line in env_file.read_text().splitlines():
        line = line.strip()
        if line.startswith("#") or "=" not in line:
            continue
        key, _, val = line.partition("=")
        val = val.strip().strip('"').strip("'")
        if key.strip() == "CYNIC_REST_ADDR" and not KERNEL_ADDR:
            KERNEL_ADDR = val
        elif key.strip() == "CYNIC_API_KEY" and not API_KEY:
            API_KEY = val


# ── Kernel POST ──

# High-signal tweets go to /judge (Dogs evaluate → crystal accumulation → CCM compounds).
# Low-signal tweets go to /observe (stored, no Dog evaluation).
JUDGE_THRESHOLD = 3

# ── Domain Assignment (narrative-first routing) ──

def load_domains_config(lab_config_dir: str = None) -> dict:
    """Load domains.yaml from lab config."""
    if lab_config_dir is None:
        lab_config_dir = Path(__file__).parent.parent.parent.parent / "cynic-python" / "lab" / "config"
    else:
        lab_config_dir = Path(lab_config_dir)

    domains_file = lab_config_dir / "domains.yaml"
    if not domains_file.exists():
        return {}

    with open(domains_file) as f:
        return yaml.safe_load(f) or {}


def load_narrative_mappings(lab_config_dir: str = None) -> dict:
    """Load narrative_domains.yaml for narrative-first domain assignment."""
    if lab_config_dir is None:
        lab_config_dir = Path(__file__).parent.parent.parent.parent / "cynic-python" / "lab" / "config"
    else:
        lab_config_dir = Path(lab_config_dir)

    narratives_file = lab_config_dir / "narrative_domains.yaml"
    if not narratives_file.exists():
        return {}

    with open(narratives_file) as f:
        data = yaml.safe_load(f) or {}
        return data.get("narrative_mappings", {})


def assign_domain(tweet: dict, domains: dict, narrative_mappings: dict) -> str:
    """Assign domain to tweet: narrative-first, then keyword, then default D1.

    Returns domain ID like 'D1', 'D2', 'D4', etc.
    """
    # Priority 1: Narrative-first assignment
    narratives = tweet.get("narratives", []) or []
    for narrative in narratives:
        for domain_id, narrative_list in narrative_mappings.items():
            if narrative in narrative_list:
                return domain_id

    # Priority 2: Keyword-based assignment (first match wins)
    text = tweet.get("text", "").lower()
    for domain_id, domain_info in domains.items():
        if not isinstance(domain_info, dict):
            continue
        keywords = domain_info.get("keywords", [])
        if any(kw.lower() in text for kw in keywords):
            return domain_id

    # Priority 3: Default fallback
    return "D1"


def _kernel_addr() -> str:
    return KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"


def _headers() -> dict:
    return {"Content-Type": "application/json", "Authorization": f"Bearer {API_KEY}"}


def build_social_stimulus(row: dict, domain: str) -> str:
    """Build structured stimulus from tweet data — mirrors kernel stimulus format.

    Dogs are calibrated on [DOMAIN]/[METRICS]/[BASELINES]/[AXIOM EVIDENCE]/[QUESTION] format.
    Prose content scores BARK because Dogs can't map unstructured text to axioms.
    K20: stimulus format = API contract with Dogs.
    """
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

    # Metrics — raw facts
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

    # Content — the actual tweet
    lines.append("[CONTENT]")
    lines.append(text)
    lines.append("")

    # Baselines
    lines.append("[BASELINES]")
    lines.append("high_signal: signal_score>=5, engagement>0.02, author_tier=whale/influencer, original content, narratives=analysis/warning")
    lines.append("moderate_signal: signal_score 3-4, engagement 0.005-0.02, verified author, some discussion")
    lines.append("noise: signal_score<3, engagement<0.005, bot tier, retweet, no cashtags/narratives")
    lines.append("coordination_risk: coordination_count>=3, identical text from multiple authors")
    lines.append("")

    # Axiom evidence
    lines.append("[AXIOM EVIDENCE]")
    lines.append("FIDELITY: Is this author's claim genuine? Does the content match observable on-chain reality?")
    lines.append("PHI: Is engagement proportional to content quality? Is follower-to-engagement ratio coherent?")
    lines.append("VERIFY: Can the claims in this tweet be verified? Are cashtags/narratives checkable on-chain?")
    lines.append("CULTURE: Does this follow established crypto discourse norms? Quality thread vs pump spam?")
    lines.append("BURN: Is the signal efficient? Original content vs noise (retweet, bot, coordinated)?")
    lines.append("SOVEREIGNTY: Is this independent analysis or coordinated shilling? Author tier vs content quality?")
    lines.append("")

    # Question
    lines.append("[QUESTION]")
    lines.append("Evaluate this social signal's reliability and information value. Score each axiom from 0.05 to 0.618.")

    return "\n".join(lines)


def post_judge(row: dict, domain: str = "D1", max_retries: int = 3) -> dict | None:
    """POST high-signal tweet to /judge → Dogs evaluate → crystal accumulation.

    Backs off on 429 (rate limit) with exponential delay. Retries up to max_retries.
    Returns the verdict dict on success, None on failure.
    """
    author = row.get("author_screen_name", "?")
    score = row.get("signal_score", 0)

    content = build_social_stimulus(row, domain)
    context = f"Passive capture via x-organ [{ACCOUNT_ID}]. Account: {ACCOUNT_ID}."

    payload = {"content": content, "context": context, "domain": domain, "account_id": ACCOUNT_ID, "priority": "hermes"}

    for attempt in range(max_retries + 1):
        try:
            resp = requests.post(
                f"{_kernel_addr()}/judge",
                json=payload,
                headers=_headers(),
                timeout=120,
            )
            if resp.status_code == 200:
                verdict = resp.json()
                logger.info("JUDGE @%s [%d]: %s Q=%.3f",
                            author, score, verdict.get("verdict", "?"),
                            verdict.get("q_score", {}).get("total", 0))
                return verdict
            if resp.status_code == 429:
                delay = 2 ** attempt * 5  # 5s, 10s, 20s, 40s
                logger.warning("POST /judge 429 rate-limited — backoff %ds (attempt %d/%d)",
                               delay, attempt + 1, max_retries + 1)
                time.sleep(delay)
                continue
            logger.warning("POST /judge %d: %s", resp.status_code, resp.text[:100])
            return None
        except requests.RequestException as e:
            logger.warning("POST /judge failed: %s", e)
            return None

    logger.warning("POST /judge exhausted %d retries for @%s", max_retries + 1, author)
    return None


def post_observe(row: dict, organ_name: str = ORGAN_NAME) -> bool:
    """POST low-signal tweet to /observe (store only, no Dog evaluation)."""
    text = row.get("text", "")[:200]
    author = row.get("author_screen_name", "?")
    score = row.get("signal_score", 0)
    tags = row.get("cashtags", []) + row.get("narratives", [])

    payload = {
        "tool": organ_name,
        "target": row.get("tweet_id", ""),
        "domain": DOMAIN,
        "status": "captured",
        "context": f"@{author} [{score}]: {text}",
        "tags": tags[:10],
        "agent_id": f"hermes-x-{ACCOUNT_ID}",
        "account_id": ACCOUNT_ID,
    }

    try:
        resp = requests.post(
            f"{_kernel_addr()}/observe",
            json=payload,
            headers=_headers(),
            timeout=5,
        )
        return resp.status_code == 200
    except requests.RequestException as e:
        logger.warning("POST /observe failed: %s", e)
        return False


def write_verdict_to_organ(row: dict, verdict: dict, organ_dir: Path) -> bool:
    """Write verdict to organ disk (idempotent, skip if exists).

    Schema:
        verdicts/{tweet_id}.json
        {
            "tweet_id": "...",
            "signal_score": N,
            "captured_at": "...",
            "verdict": {...},
            "written_at": "ISO8601"
        }
    """
    tweet_id = row.get("tweet_id", "")
    if not tweet_id:
        return False

    verdict_dir = organ_dir / "verdicts"
    try:
        verdict_dir.mkdir(parents=True, exist_ok=True)
    except OSError as e:
        logger.warning("Failed to create verdicts directory: %s", e)
        return False

    verdict_file = verdict_dir / f"{tweet_id}.json"

    # Idempotent: skip if already exists
    if verdict_file.exists():
        return True

    try:
        import datetime
        payload = {
            "tweet_id": tweet_id,
            "signal_score": row.get("signal_score", 0),
            "captured_at": row.get("capture_ts", ""),
            "verdict": verdict,
            "written_at": datetime.datetime.utcnow().isoformat() + "Z",
        }
        verdict_file.write_text(json.dumps(payload, indent=2))
        return True
    except OSError as e:
        logger.warning("Failed to write verdict for %s: %s", tweet_id, e)
        return False


def ingest_row(row: dict, organ_name: str = ORGAN_NAME, organ_dir: Path = DEFAULT_ORGAN_DIR,
               domains: dict = None, narrative_mappings: dict = None) -> bool:
    """Route tweet: high-signal → /judge + write to organ, low-signal → /observe.

    High-signal tweets are routed to /judge with narrative-first domain assignment.
    Throttles /judge calls to avoid rate limiting (Dogs need inference time).
    """
    score = row.get("signal_score", 0)
    if score >= JUDGE_THRESHOLD:
        # Assign domain (narrative-first, then keyword, then D1 fallback)
        domain = assign_domain(row, domains or {}, narrative_mappings or {})
        time.sleep(JUDGE_THROTTLE)  # Respect inference capacity
        verdict = post_judge(row, domain)
        if verdict:
            write_verdict_to_organ(row, verdict, organ_dir)
            return True
        return False
    return post_observe(row, organ_name)


# ── Organ heartbeat ──

_heartbeat_sent = 0
_heartbeat_errors = 0


def detect_auth_failure(dataset: Path, rows_to_check: int = 10) -> bool:
    """Detect extraction failure if recent tweets lack structural data.

    X.com returns auth error page when session expires → proxy can't parse tweets.
    Symptom: tweets with empty tweet_id or text (not low engagement_rate).
    (Engagement is data quality, not health. Low engagement on fresh tweets is normal.)
    """
    if not dataset.exists():
        return False
    try:
        with open(dataset, "r") as f:
            # Seek to near end and read last N lines
            f.seek(0, 2)  # Go to end
            file_size = f.tell()
            if file_size == 0:
                return False
            # Read last chunk
            chunk_size = min(file_size, 65536)  # Read last 64KB
            f.seek(max(0, file_size - chunk_size))
            content = f.read()

        lines = [l.strip() for l in content.split('\n') if l.strip()][-rows_to_check:]
        if not lines:
            return False

        # Check if recent tweets have valid identity fields
        zero_identity = 0
        for line in lines:
            try:
                row = json.loads(line)
                if not row.get("tweet_id") or not row.get("text"):
                    zero_identity += 1
            except json.JSONDecodeError:
                continue

        # If >50% of recent tweets lack identity, suspect extraction/auth failure
        return zero_identity >= len(lines) // 2
    except Exception as e:
        logger.warning("detect_auth_failure error: %s", e)
        return False


def post_heartbeat(organ_name: str, dataset: Path) -> bool:
    """POST organ heartbeat to /observe — tells the kernel this organ is alive."""
    global _heartbeat_sent, _heartbeat_errors

    # Check if dataset is growing (proxy is capturing)
    try:
        size = dataset.stat().st_size
        mtime = dataset.stat().st_mtime
        age_secs = time.time() - mtime
        dataset_growing = age_secs < 300  # modified in last 5 min
    except OSError:
        size = 0
        age_secs = -1
        dataset_growing = False

    # Check if proxy is listening
    import socket
    proxy_up = False
    try:
        with socket.create_connection(("127.0.0.1", 8888), timeout=1):
            proxy_up = True
    except (ConnectionRefusedError, OSError):
        pass

    # Check if Chrome CDP is responding
    cdp_up = False
    try:
        resp = requests.get("http://127.0.0.1:40769/json/version", timeout=2)
        cdp_up = resp.status_code == 200
    except requests.RequestException:
        pass

    # Check for auth failure
    auth_failed = not dataset_growing and detect_auth_failure(dataset)

    status = "ok"
    failure_reason = None
    if not proxy_up:
        status = "critical"
        failure_reason = "proxy_down"
    elif not cdp_up:
        status = "degraded"
        failure_reason = "cdp_down"
    elif auth_failed:
        status = "critical"
        failure_reason = "x_auth_expired"
    elif not dataset_growing:
        status = "degraded"
        failure_reason = "dataset_stale"

    context = (
        f"sent={_heartbeat_sent} errors={_heartbeat_errors} "
        f"dataset_bytes={size} dataset_age_secs={int(age_secs)} "
        f"dataset_growing={dataset_growing} "
        f"proxy_up={proxy_up} cdp_up={cdp_up} auth_failed={auth_failed}"
    )

    payload = {
        "tool": "heartbeat",
        "target": organ_name,
        "domain": "organ-health",
        "status": status,
        "context": context,
        "tags": ["heartbeat", organ_name, f"account-{ACCOUNT_ID}"],
        "agent_id": f"hermes-{organ_name}",
        "account_id": ACCOUNT_ID,
    }
    if failure_reason:
        payload["failure_reason"] = failure_reason
    if status == "critical":
        payload["consumer"] = f"hermes-x-recovery@{ACCOUNT_ID}"

    try:
        resp = requests.post(
            f"{_kernel_addr()}/observe",
            json=payload,
            headers=_headers(),
            timeout=5,
        )
        if resp.status_code == 200:
            logger.debug("heartbeat: %s (%s)", status, context)
            return True
        logger.warning("heartbeat POST %d", resp.status_code)
        return False
    except requests.RequestException as e:
        logger.warning("heartbeat failed: %s", e)
        return False


# ── State file for cursor persistence ──

def load_cursor(state_path: Path) -> int:
    if state_path.exists():
        try:
            return int(state_path.read_text().strip())
        except (ValueError, OSError):
            pass
    return 0


def save_cursor(state_path: Path, offset: int):
    try:
        state_path.write_text(str(offset))
    except OSError:
        pass


# ── Tail loop ──

def tail_dataset(dataset: Path, state_path: Path, organ_name: str = ORGAN_NAME, organ_dir: Path = DEFAULT_ORGAN_DIR, replay: bool = False):
    """Tail the dataset JSONL, POST new lines to /observe, write verdicts to organ.

    High-signal tweets are routed to /judge with narrative-first domain assignment.
    """
    if not dataset.exists():
        logger.info("Dataset not found: %s — waiting for creation", dataset)
        while not dataset.exists():
            time.sleep(POLL_INTERVAL)

    # Load domain assignment config
    domains = load_domains_config()
    narrative_mappings = load_narrative_mappings()
    if domains:
        logger.info("Loaded %d domains from narrative_domains.yaml", len(narrative_mappings))
    else:
        logger.warning("No domains config found; will default to D1 for all high-signal tweets")

    offset = 0 if replay else load_cursor(state_path)
    if not replay and offset == 0:
        # First run: seek to end (don't replay history)
        offset = dataset.stat().st_size
        save_cursor(state_path, offset)
        logger.info("First run — seeking to end (offset %d)", offset)

    logger.info("Tailing %s from offset %d (replay=%s) — organ_dir=%s", dataset, offset, replay, organ_dir)

    global _heartbeat_sent, _heartbeat_errors
    sent = 0
    errors = 0
    last_heartbeat = 0.0
    while True:
        # Organ heartbeat — self-diagnosis every 60s
        now = time.time()
        if not replay and now - last_heartbeat >= HEARTBEAT_INTERVAL:
            _heartbeat_sent = sent
            _heartbeat_errors = errors
            post_heartbeat(organ_name, dataset)
            last_heartbeat = now
        try:
            size = dataset.stat().st_size
        except OSError:
            time.sleep(POLL_INTERVAL)
            continue

        if size <= offset:
            if not replay:
                time.sleep(POLL_INTERVAL)
                continue
            else:
                break

        batch = []
        with open(dataset) as f:
            f.seek(offset)
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    batch.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
                if len(batch) >= BATCH_SIZE:
                    for row in batch:
                        if ingest_row(row, organ_name, organ_dir, domains, narrative_mappings):
                            sent += 1
                        else:
                            errors += 1
                    batch = []
            new_offset = f.tell()

        # Flush remaining
        for row in batch:
            if ingest_row(row, organ_name, organ_dir, domains, narrative_mappings):
                sent += 1
            else:
                errors += 1

        offset = new_offset
        save_cursor(state_path, offset)

        if replay:
            break

        if sent % 100 == 0 and sent > 0:
            logger.info("Progress: %d sent, %d errors", sent, errors)

    logger.info("Done: %d sent, %d errors", sent, errors)


# ── Main ──

def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )
    load_env()
    if not KERNEL_ADDR:
        logger.error("CYNIC_REST_ADDR not set")
        sys.exit(1)

    # Account validation: best-effort, not blocking.
    # The ingest daemon POSTs to the kernel, not to X.com — it doesn't need X credentials.
    # The proxy (x_proxy.py) handles X auth. Guard removed after restart crash (2026-05-16).
    try:
        username, _ = get_x_credentials(interactive=False)
        logger.info("Account validation passed: %s", username)
    except RuntimeError as e:
        logger.warning("Account config not available (non-blocking): %s", e)

    parser = argparse.ArgumentParser(description="CYNIC ingest daemon")
    parser.add_argument("--dataset", type=Path, default=DEFAULT_DATASET)
    parser.add_argument("--organ-dir", type=Path, default=DEFAULT_ORGAN_DIR,
                        help="Organ directory for verdict storage (X_ORGAN_DIR env var)")
    parser.add_argument("--replay", action="store_true",
                        help="Replay full dataset (one-shot, no tail)")
    parser.add_argument("--organ", default=ORGAN_NAME,
                        help="Organ name for /observe tool field")
    args = parser.parse_args()

    organ = args.organ
    organ_dir = args.organ_dir

    state_path = args.dataset.parent / "ingest_cursor.txt"

    # Graceful shutdown
    running = True
    def handle_signal(sig, frame):
        nonlocal running
        running = False
        logger.info("Shutting down (signal %s)", sig)
    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)

    logger.info("Ingest daemon starting — organ=%s dataset=%s organ_dir=%s kernel=%s",
                ORGAN_NAME, args.dataset, organ_dir, KERNEL_ADDR)

    tail_dataset(args.dataset, state_path, organ_name=organ, organ_dir=organ_dir, replay=args.replay)


if __name__ == "__main__":
    main()
