"""Runtime implementation for the Hermes X recovery daemon."""

from __future__ import annotations

import logging
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Optional

from organs.contracts.src.adapters.kernel_http import normalize_kernel_base_url
from organs.contracts.src.adapters.kernel_observe import post_kernel_observe
from organs.contracts.src.project_root import resolve_repo_root
from organs.contracts.src.surface_contracts import iter_json_objects
from organs.twitter.src.adapters.get_x_credentials import get_x_credentials
from organs.twitter.src.adapters.hermes_paths import DATASET, RECOVERY_LOG


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
ACCOUNT_ID = os.getenv("HERMES_ACCOUNT", "cynic")
REST_ADDR = ""
API_KEY = os.getenv("CYNIC_API_KEY", "")
CHECK_INTERVAL = int(os.getenv("RECOVERY_CHECK_INTERVAL", "30"))
LOG_PATH = Path(os.getenv("RECOVERY_LOG_PATH", str(RECOVERY_LOG)))
FAILURE_COUNT = 0
MAX_FAILURE_BEFORE_LAYER2 = 2
LAYER2_RETRY_COUNT = 0
MAX_LAYER2_RETRIES = 3
LAYER3_RETRY_COUNT = 0


def _normalize_kernel_addr(raw: str) -> str:
    try:
        return normalize_kernel_base_url(raw, allow_empty=True)
    except RuntimeError:
        return ""


REST_ADDR = _normalize_kernel_addr(os.getenv("CYNIC_REST_ADDR", ""))


def configure_logger() -> logging.Logger:
    """Configure logger without failing when filesystem is read-only."""
    logger = logging.getLogger(__name__)
    if logger.handlers:
        return logger

    logger.setLevel(logging.INFO)
    logger.propagate = False
    formatter = logging.Formatter("%(asctime)s [%(levelname)s] %(message)s")
    stream_handler = logging.StreamHandler(sys.stdout)
    stream_handler.setFormatter(formatter)
    logger.addHandler(stream_handler)

    try:
        LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        file_handler = logging.FileHandler(LOG_PATH)
        file_handler.setFormatter(formatter)
        logger.addHandler(file_handler)
    except OSError as exc:
        logger.warning("Recovery logger file sink unavailable; using stdout only: %s", exc)
    return logger


logger = configure_logger()


def get_accounts_config() -> dict[str, Any]:
    """Load accounts.toml from ~/.config/cynic/."""
    config_path = Path.home() / ".config/cynic/accounts.toml"
    if not config_path.exists():
        return {}
    try:
        import tomllib

        with config_path.open("rb") as handle:
            return tomllib.load(handle)
    except Exception as exc:
        logger.error("Failed to load accounts.toml: %s", exc)
        return {}


def check_heartbeat() -> Optional[dict[str, Any]]:
    """Check auth health by reading the dataset directly."""
    dataset = Path(os.getenv("X_DATASET_PATH") or str(DATASET))
    if not dataset.exists():
        return {"status": "critical", "failure_reason": "dataset_missing"}

    age_secs = time.time() - dataset.stat().st_mtime
    if age_secs < 300:
        return {"status": "ok", "failure_reason": None}

    try:
        with dataset.open("rb") as handle:
            handle.seek(max(0, dataset.stat().st_size - 65536))
            raw = handle.read().decode("utf-8", errors="replace")
            lines = [line for line in raw.splitlines() if line.strip()][-10:]
    except OSError:
        return {"status": "degraded", "failure_reason": "dataset_read_error"}

    zero_identity = 0
    for row in iter_json_objects(lines):
        if not row.get("tweet_id") or not row.get("text"):
            zero_identity += 1

    if lines and zero_identity >= len(lines) // 2:
        return {"status": "critical", "failure_reason": "x_extraction_broken"}
    return {"status": "degraded", "failure_reason": "dataset_stale"}


def layer2_retry_login() -> bool:
    """Attempt to re-authenticate by running hermes_x_login.py --force."""
    global LAYER2_RETRY_COUNT
    LAYER2_RETRY_COUNT += 1
    logger.info("[Layer 2] Retrying login (attempt %d/%d)", LAYER2_RETRY_COUNT, MAX_LAYER2_RETRIES)

    try:
        script_path = PROJECT_ROOT / "organs" / "twitter" / "src" / "entrypoints" / "hermes_x_login.py"
        if not script_path.exists():
            logger.error("Login script not found: %s", script_path)
            return False

        env = os.environ.copy()
        env["HERMES_ACCOUNT"] = ACCOUNT_ID
        result = subprocess.run(
            ["python3", str(script_path), "--force"],
            env=env,
            timeout=120,
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            logger.info("[Layer 2] Re-login successful for %s", ACCOUNT_ID)
            return True
        logger.warning("[Layer 2] Re-login failed: %s", result.stderr)
        return False
    except subprocess.TimeoutExpired:
        logger.error("[Layer 2] Login timeout")
        return False
    except Exception as exc:
        logger.error("[Layer 2] Error running login: %s", exc)
        return False


def get_fallback_account() -> Optional[str]:
    """Get the fallback account ID from accounts.toml if configured."""
    try:
        config = get_accounts_config()
        if "accounts" not in config:
            return None
        for account_id, account_cfg in config["accounts"].items():
            if account_id != ACCOUNT_ID and account_cfg.get("resume_on_failure", False):
                return account_id
        return None
    except Exception as exc:
        logger.error("Error reading fallback account: %s", exc)
        return None


def layer3_switch_account(fallback_account: str) -> bool:
    """Switch to fallback account via organ-owned Python entrypoint."""
    global LAYER3_RETRY_COUNT
    LAYER3_RETRY_COUNT += 1
    logger.info("[Layer 3] Switching to fallback account: %s", fallback_account)

    try:
        entrypoint = PROJECT_ROOT / "organs" / "twitter" / "src" / "entrypoints" / "toggle_account.py"
        if not entrypoint.exists():
            logger.error("Toggle entrypoint not found: %s", entrypoint)
            return False
        result = subprocess.run(
            [sys.executable, str(entrypoint), fallback_account],
            timeout=30,
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            logger.info("[Layer 3] Successfully switched to %s", fallback_account)
            return True
        logger.error("[Layer 3] Switch failed: %s", result.stderr)
        return False
    except subprocess.TimeoutExpired:
        logger.error("[Layer 3] Switch timeout")
        return False
    except Exception as exc:
        logger.error("[Layer 3] Error switching account: %s", exc)
        return False


def layer4_alert_kernel():
    """Post critical alert to kernel because all recovery attempts failed."""
    logger.critical("[Layer 4] All recovery attempts failed for %s. Alerting kernel.", ACCOUNT_ID)
    try:
        import requests

        payload = {
            "tool": "hermes-recovery",
            "domain": "organ-health",
            "target": ACCOUNT_ID,
            "status": "critical",
            "context": (
                f"account={ACCOUNT_ID} layer2_retries={LAYER2_RETRY_COUNT} "
                f"layer3_retries={LAYER3_RETRY_COUNT} reason=x_auth_expired "
                "manual_intervention_required=true"
            ),
            "consumer": f"human@{ACCOUNT_ID}",
            "agent_id": f"hermes-recovery-{ACCOUNT_ID}",
            "tags": ["recovery-failed", f"account-{ACCOUNT_ID}"],
        }
        result = post_kernel_observe(
            request_post=requests.post,
            kernel_url=REST_ADDR,
            auth_key=API_KEY,
            payload=payload,
            timeout=10,
        )
        if result.posted:
            logger.info("[Layer 4] Critical alert sent to kernel")
        else:
            logger.error("[Layer 4] Failed to alert kernel: %s", result.error or result.status_code)
    except Exception as exc:
        logger.error("[Layer 4] Error alerting kernel: %s", exc)


def run_recovery_cycle():
    """Check heartbeat and handle failures."""
    global FAILURE_COUNT, LAYER2_RETRY_COUNT, LAYER3_RETRY_COUNT
    heartbeat = check_heartbeat()
    if not heartbeat:
        logger.debug("No heartbeat data available")
        return

    if heartbeat.get("status") == "critical" and heartbeat.get("failure_reason") == "x_auth_expired":
        FAILURE_COUNT += 1
        logger.warning("Auth failure detected (%d/%d)", FAILURE_COUNT, MAX_FAILURE_BEFORE_LAYER2)
        if FAILURE_COUNT >= MAX_FAILURE_BEFORE_LAYER2 and LAYER2_RETRY_COUNT == 0:
            if layer2_retry_login():
                logger.info("Recovery successful at Layer 2. Resetting failure counters.")
                FAILURE_COUNT = 0
                LAYER2_RETRY_COUNT = 0
                return
            if LAYER2_RETRY_COUNT >= MAX_LAYER2_RETRIES:
                fallback = get_fallback_account()
                if fallback and layer3_switch_account(fallback):
                    logger.info("Successfully switched to fallback account.")
                    FAILURE_COUNT = 0
                    LAYER2_RETRY_COUNT = 0
                    LAYER3_RETRY_COUNT = 0
                    return
                if not fallback:
                    logger.warning("No fallback account configured. Escalating to Layer 4.")
                layer4_alert_kernel()
    else:
        if FAILURE_COUNT > 0:
            logger.info("Heartbeat healthy. Resetting failure counters.")
        FAILURE_COUNT = 0
        LAYER2_RETRY_COUNT = 0
        LAYER3_RETRY_COUNT = 0


def main():
    """Run the recovery daemon."""
    logger.info("Hermes X Recovery Daemon started for account: %s", ACCOUNT_ID)
    if not REST_ADDR:
        logger.error("CYNIC_REST_ADDR is required and must be non-loopback.")
        sys.exit(1)
    logger.info("Kernel: %s", REST_ADDR)
    logger.info("Check interval: %ss", CHECK_INTERVAL)

    try:
        username, _ = get_x_credentials(interactive=False)
        logger.info("Account validation passed: %s", username)
    except RuntimeError as exc:
        logger.error("Account configuration invalid: %s", exc)
        sys.exit(1)

    try:
        while True:
            try:
                run_recovery_cycle()
            except Exception as exc:
                logger.error("Error in recovery cycle: %s", exc, exc_info=True)
            time.sleep(CHECK_INTERVAL)
    except KeyboardInterrupt:
        logger.info("Recovery daemon stopped by user")
        sys.exit(0)
    except Exception as exc:
        logger.critical("Unrecoverable error: %s", exc, exc_info=True)
        sys.exit(1)
