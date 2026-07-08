"""Runtime implementation for the Hermes X ingest daemon."""

from __future__ import annotations

import argparse
import logging
import os
import signal
import sys
import time
from datetime import datetime
from pathlib import Path

import requests
import yaml

from organs.contracts.src.adapters.env import load_env_file
from organs.contracts.src.project_root import resolve_repo_root
from organs.contracts.src.surface_contracts import json_loads_or_default
from organs.twitter.src.adapters.get_x_credentials import get_x_credentials
from organs.twitter.src.adapters.hermes_paths import DATASET as DEFAULT_DATASET
from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR as DEFAULT_ORGAN_DIR
from organs.twitter.src.adapters.x_ingest_health import post_heartbeat
from organs.twitter.src.adapters.x_ingest_store import load_cursor, save_cursor
from organs.twitter.src.adapters.x_kernel_transport import (
    KernelTransport,
    build_convergence_payload,
    build_social_judgment_payload,
    build_token_judgment_payload,
    build_tweet_observe_payload,
)
from organs.twitter.src.core.convergence_detector import detect_convergence
from organs.twitter.src.core.ingest_signals import assign_domain


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
for resolver_path in (
    PROJECT_ROOT / "services" / "cynic-python" / "resolvers",
    PROJECT_ROOT / "cynic-python" / "resolvers",
):
    if resolver_path.exists() and str(resolver_path) not in sys.path:
        sys.path.insert(0, str(resolver_path))

from cashtag_mint import CashtagResolver


logger = logging.getLogger("x-ingest")
ACCOUNT_ID = os.environ.get("HERMES_ACCOUNT", "cynic")
KERNEL_ADDR = ""
API_KEY = ""
ORGAN_NAME = f"x-proxy-{ACCOUNT_ID}"
DOMAIN = "twitter"
POLL_INTERVAL = 2.0
BATCH_SIZE = 20
HEARTBEAT_INTERVAL = 60.0
JUDGE_THRESHOLD = 3
CONVERGENCE_INTERVAL = 50
CONVERGENCE_COOLDOWN = 3600
_obs_buffer: list[dict] = []
_convergence_cooldown: dict[str, float] = {}
_resolver: CashtagResolver | None = None


def load_env():
    """Load Kernel coordinates from environment or ~/.cynic-env."""
    global KERNEL_ADDR, API_KEY
    KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = os.environ.get("CYNIC_API_KEY", "")
    if KERNEL_ADDR and API_KEY:
        return
    load_env_file(Path.home() / ".cynic-env", allowed_keys=("CYNIC_REST_ADDR", "CYNIC_API_KEY"))
    KERNEL_ADDR = KERNEL_ADDR or os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = API_KEY or os.environ.get("CYNIC_API_KEY", "")


def load_domains_config(lab_config_dir: str | Path | None = None) -> dict:
    """Load domains.yaml from lab config."""
    config_dir = Path(lab_config_dir) if lab_config_dir is not None else PROJECT_ROOT / "services" / "cynic-python" / "lab" / "config"
    domains_file = config_dir / "domains.yaml"
    if not domains_file.exists():
        return {}
    with domains_file.open(encoding="utf-8") as handle:
        return yaml.safe_load(handle) or {}


def load_narrative_mappings(lab_config_dir: str | Path | None = None) -> dict:
    """Load narrative_domains.yaml for narrative-first domain assignment."""
    config_dir = Path(lab_config_dir) if lab_config_dir is not None else PROJECT_ROOT / "services" / "cynic-python" / "lab" / "config"
    narratives_file = config_dir / "narrative_domains.yaml"
    if not narratives_file.exists():
        return {}
    with narratives_file.open(encoding="utf-8") as handle:
        data = yaml.safe_load(handle) or {}
    return data.get("narrative_mappings", {})


def _transport() -> KernelTransport:
    return KernelTransport(KERNEL_ADDR, API_KEY)


def post_judge(row: dict, domain: str = "D1", max_retries: int = 3) -> dict | None:
    """POST high-signal tweet to /judge."""
    author = row.get("author_screen_name", "?")
    score = row.get("signal_score", 0)
    payload = build_social_judgment_payload(row, domain, ACCOUNT_ID)
    for attempt in range(max_retries + 1):
        try:
            resp = _transport().post_json("/judge", payload, timeout=120)
            if resp.status_code == 200:
                verdict = resp.json()
                logger.info(
                    "JUDGE @%s [%d]: %s Q=%.3f",
                    author,
                    score,
                    verdict.get("verdict", "?"),
                    verdict.get("q_score", {}).get("total", 0),
                )
                return verdict
            if resp.status_code == 429:
                delay = 2 ** attempt * 5
                logger.warning("POST /judge 429 rate-limited - backoff %ds", delay)
                time.sleep(delay)
                continue
            logger.warning("POST /judge %d: %s", resp.status_code, resp.text[:100])
            return None
        except requests.RequestException as exc:
            logger.warning("POST /judge failed: %s", exc)
            return None
    logger.warning("POST /judge exhausted %d retries for @%s", max_retries + 1, author)
    return None


def post_observe(row: dict, organ_name: str = ORGAN_NAME, domain: str = DOMAIN) -> bool:
    """POST tweet to /observe."""
    payload = build_tweet_observe_payload(row, organ_name, domain, ACCOUNT_ID)
    try:
        return _transport().post_observe(payload, timeout=5)
    except requests.RequestException as exc:
        logger.warning("POST /observe failed: %s", exc)
        return False


def emit_convergence(signals: list, resolver: CashtagResolver) -> int:
    """POST convergence summaries to /observe."""
    emitted = 0
    now = time.time()
    for signal in signals:
        if signal.cashtag in _convergence_cooldown and now - _convergence_cooldown[signal.cashtag] < CONVERGENCE_COOLDOWN:
            continue
        signal.resolved_mint = resolver.resolve(signal.cashtag)
        payload = build_convergence_payload(signal, ACCOUNT_ID)
        try:
            if _transport().post_observe(payload, timeout=10):
                _convergence_cooldown[signal.cashtag] = now
                emitted += 1
                logger.info("CONVERGENCE %s: %d authors -> %s", signal.cashtag, signal.author_count, payload["target"])
                if signal.resolved_mint:
                    _trigger_token_judgment(signal)
        except requests.RequestException as exc:
            logger.warning("convergence POST failed: %s", exc)
    return emitted


def _trigger_token_judgment(signal) -> None:
    judge_payload = build_token_judgment_payload(signal)
    try:
        result = _transport().post_judge_payload(judge_payload, timeout=30)
        if result is not None:
            logger.info(
                "JUDGE %s ($%s): %s Q=%.3f",
                signal.resolved_mint[:8],
                signal.cashtag,
                result.get("verdict", "?"),
                result.get("q_score", {}).get("total", 0),
            )
        else:
            logger.warning("judge POST for %s failed", signal.cashtag)
    except requests.RequestException as exc:
        logger.warning("judge POST failed for %s: %s", signal.cashtag, exc)


def ingest_row(
    row: dict,
    organ_name: str = ORGAN_NAME,
    organ_dir: Path = DEFAULT_ORGAN_DIR,
    domains: dict | None = None,
    narrative_mappings: dict | None = None,
) -> bool:
    """Route tweet to /observe with narrative-first domain assignment."""
    domain = assign_domain(row, domains or {}, narrative_mappings or {})
    _obs_buffer.append(
        {
            "agent_id": f"hermes-{ACCOUNT_ID}",
            "tags": (row.get("cashtags", []) or []) + (row.get("narratives", []) or []),
            "context": f"@{row.get('author_screen_name', '?')} [{row.get('signal_score', 0)}]: {row.get('text', '')[:200]}",
            "created_at": row.get("capture_ts", datetime.now().isoformat()),
            "target": row.get("tweet_id", ""),
        }
    )
    if len(_obs_buffer) > 500:
        _obs_buffer[:] = _obs_buffer[-500:]
    return post_observe(row, organ_name, domain)


def tail_dataset(
    dataset: Path,
    state_path: Path,
    organ_name: str = ORGAN_NAME,
    organ_dir: Path = DEFAULT_ORGAN_DIR,
    replay: bool = False,
):
    """Tail the dataset JSONL, POST new rows to /observe."""
    if not dataset.exists():
        logger.info("Dataset not found: %s - waiting for creation", dataset)
        while not dataset.exists():
            time.sleep(POLL_INTERVAL)

    domains = load_domains_config()
    narrative_mappings = load_narrative_mappings()
    if domains:
        logger.info("Loaded %d narrative mappings", len(narrative_mappings))
    else:
        logger.warning("No domains config found; will default to D1")

    global _resolver
    _resolver = CashtagResolver.load(organ_dir / "cashtag_cache.json")
    logger.info("Loaded cashtag resolver (%d entries)", len(_resolver._cache))

    offset = 0 if replay else load_cursor(state_path)
    if not replay and offset == 0:
        offset = dataset.stat().st_size
        save_cursor(state_path, offset)
        logger.info("First run - seeking to end (offset %d)", offset)

    logger.info("Tailing %s from offset %d (replay=%s) - organ_dir=%s", dataset, offset, replay, organ_dir)
    sent = 0
    errors = 0
    last_heartbeat = 0.0
    while True:
        now = time.time()
        if not replay and now - last_heartbeat >= HEARTBEAT_INTERVAL:
            post_heartbeat(
                transport=_transport(),
                organ_name=organ_name,
                account_id=ACCOUNT_ID,
                dataset=dataset,
                sent=sent,
                errors=errors,
            )
            last_heartbeat = now

        try:
            size = dataset.stat().st_size
        except OSError:
            time.sleep(POLL_INTERVAL)
            continue

        if size <= offset:
            if replay:
                break
            time.sleep(POLL_INTERVAL)
            continue

        batch = []
        with dataset.open(encoding="utf-8") as handle:
            handle.seek(offset)
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                row = json_loads_or_default(line, None)
                if row is None:
                    continue
                batch.append(row)
                if len(batch) >= BATCH_SIZE:
                    sent, errors = _flush_batch(batch, sent, errors, organ_name, organ_dir, domains, narrative_mappings)
                    batch = []
            new_offset = handle.tell()

        sent, errors = _flush_batch(batch, sent, errors, organ_name, organ_dir, domains, narrative_mappings)
        offset = new_offset
        save_cursor(state_path, offset)

        if sent > 0 and sent % CONVERGENCE_INTERVAL == 0 and not replay:
            signals = detect_convergence(_obs_buffer, min_authors=3, window_hours=6)
            if signals and _resolver:
                count = emit_convergence(signals, _resolver)
                if count > 0:
                    logger.info("Convergence: %d signals emitted from %d observations", count, len(_obs_buffer))

        if replay:
            break
        if sent % 100 == 0 and sent > 0:
            logger.info("Progress: %d sent, %d errors", sent, errors)

    logger.info("Done: %d sent, %d errors", sent, errors)


def _flush_batch(
    batch: list[dict],
    sent: int,
    errors: int,
    organ_name: str,
    organ_dir: Path,
    domains: dict,
    narrative_mappings: dict,
) -> tuple[int, int]:
    for row in batch:
        if ingest_row(row, organ_name, organ_dir, domains, narrative_mappings):
            sent += 1
        else:
            errors += 1
    return sent, errors


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CYNIC ingest daemon")
    parser.add_argument("--dataset", type=Path, default=DEFAULT_DATASET)
    parser.add_argument("--organ-dir", type=Path, default=DEFAULT_ORGAN_DIR)
    parser.add_argument("--replay", action="store_true")
    parser.add_argument("--organ", default=ORGAN_NAME)
    return parser


def main():
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
    load_env()
    if not KERNEL_ADDR:
        logger.error("CYNIC_REST_ADDR not set")
        sys.exit(1)

    try:
        username, _ = get_x_credentials(interactive=False)
        logger.info("Account validation passed: %s", username)
    except RuntimeError as exc:
        logger.warning("Account config not available (non-blocking): %s", exc)

    args = build_parser().parse_args()
    state_path = args.dataset.parent / "ingest_cursor.txt"

    def handle_signal(sig, frame):
        logger.info("Shutting down (signal %s)", sig)

    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)
    logger.info("Ingest daemon starting - organ=%s dataset=%s organ_dir=%s kernel=%s", ORGAN_NAME, args.dataset, args.organ_dir, KERNEL_ADDR)
    tail_dataset(args.dataset, state_path, organ_name=args.organ, organ_dir=args.organ_dir, replay=args.replay)
