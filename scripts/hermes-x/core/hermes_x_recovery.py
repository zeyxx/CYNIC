#!/usr/bin/env python3
"""
Hermes X Recovery Daemon — monitors auth failures and implements recovery cascade.

Monitors heartbeat status for a specific X account. When auth failure is detected,
triggers a 4-layer recovery cascade:

Layer 1 (Detect): heartbeat reports failure_reason="x_auth_expired" for 2 cycles
Layer 2 (Retry): run hermes_x_login.py --force to re-authenticate
Layer 3 (Fallback): if Layer 2 fails & fallback account exists, switch accounts
Layer 4 (Alert): if all recovery attempts fail, alert kernel with critical severity

Usage:
    hermes-x-recovery.py

Environment variables (required):
    HERMES_ACCOUNT   - Account ID (cynic or personal)
    CYNIC_REST_ADDR  - Kernel REST endpoint
    CYNIC_API_KEY    - Bearer token for kernel auth

Optional:
    RECOVERY_CHECK_INTERVAL - Seconds between checks (default: 30)
    RECOVERY_LOG_PATH       - Path to recovery.log (default: ~/.cynic/organs/hermes/x/recovery.log)
"""

import os
import sys
import time
import json
import subprocess
import logging
from pathlib import Path
from datetime import datetime
from typing import Optional, Dict, Any

# Configuration
ACCOUNT_ID = os.getenv("HERMES_ACCOUNT", "cynic")
REST_ADDR = os.getenv("CYNIC_REST_ADDR", "http://127.0.0.1:3030")
API_KEY = os.getenv("CYNIC_API_KEY", "")
CHECK_INTERVAL = int(os.getenv("RECOVERY_CHECK_INTERVAL", "30"))
from hermes_paths import HERMES_X_DIR as ORGAN_DIR, RECOVERY_LOG
from get_x_credentials import get_x_credentials
LOG_PATH = Path(os.getenv("RECOVERY_LOG_PATH", str(RECOVERY_LOG)))

# Recovery state
FAILURE_COUNT = 0
MAX_FAILURE_BEFORE_LAYER2 = 2  # 2 heartbeat cycles = ~60s
LAYER2_RETRY_COUNT = 0
MAX_LAYER2_RETRIES = 3
LAYER3_RETRY_COUNT = 0

# Logging
LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[
        logging.FileHandler(LOG_PATH),
        logging.StreamHandler(sys.stdout),
    ],
)
logger = logging.getLogger(__name__)


def get_accounts_config() -> Dict[str, Any]:
    """Load accounts.toml from ~/.config/cynic/."""
    config_path = Path.home() / ".config/cynic/accounts.toml"
    if not config_path.exists():
        return {}
    try:
        import tomllib
        with open(config_path, "rb") as f:
            return tomllib.load(f)
    except Exception as e:
        logger.error(f"Failed to load accounts.toml: {e}")
        return {}


def check_heartbeat() -> Optional[Dict[str, Any]]:
    """
    Check auth health by reading the dataset directly.
    Mirrors x_ingest_daemon.detect_auth_failure() logic.
    Returns dict: {status, failure_reason} or None if dataset missing.
    """
    dataset_path_str = os.getenv("X_DATASET_PATH")
    if not dataset_path_str:
        from hermes_paths import DATASET
        dataset_path_str = str(DATASET)
    dataset = Path(dataset_path_str)

    if not dataset.exists():
        return {"status": "critical", "failure_reason": "dataset_missing"}

    # Stale check: not modified in last 5 min
    age_secs = time.time() - dataset.stat().st_mtime
    if age_secs < 300:
        return {"status": "ok", "failure_reason": None}

    # Extraction quality: recent rows must have valid tweet_id and text
    # (NOT engagement_rate — low engagement is normal for fresh tweets, not a health signal)
    # Real auth failure = X returns error page → no tweet_legacy object → empty tweet_id/text
    zero_identity = 0
    lines = []
    try:
        with open(dataset, "rb") as f:
            # Read last 64KB
            f.seek(max(0, dataset.stat().st_size - 65536))
            raw = f.read().decode("utf-8", errors="replace")
            lines = [l for l in raw.splitlines() if l.strip()][-10:]
    except OSError:
        return {"status": "degraded", "failure_reason": "dataset_read_error"}

    for line in lines:
        try:
            row = json.loads(line)
            if not row.get("tweet_id") or not row.get("text"):
                zero_identity += 1
        except json.JSONDecodeError:
            continue

    if lines and zero_identity >= len(lines) // 2:
        return {"status": "critical", "failure_reason": "x_extraction_broken"}

    return {"status": "degraded", "failure_reason": "dataset_stale"}


def layer2_retry_login() -> bool:
    """
    Layer 2: Attempt to re-authenticate by running hermes_x_login.py --force.

    Returns True if login succeeded, False otherwise.
    """
    global LAYER2_RETRY_COUNT

    LAYER2_RETRY_COUNT += 1
    logger.info(f"[Layer 2] Retrying login (attempt {LAYER2_RETRY_COUNT}/{MAX_LAYER2_RETRIES})")

    try:
        # Run hermes_x_login.py --force
        script_path = Path(__file__).parent / "hermes_x_login.py"
        if not script_path.exists():
            logger.error(f"Login script not found: {script_path}")
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
            logger.info(f"[Layer 2] Re-login successful for {ACCOUNT_ID}")
            return True
        else:
            logger.warning(f"[Layer 2] Re-login failed: {result.stderr}")
            return False

    except subprocess.TimeoutExpired:
        logger.error(f"[Layer 2] Login timeout")
        return False
    except Exception as e:
        logger.error(f"[Layer 2] Error running login: {e}")
        return False


def get_fallback_account() -> Optional[str]:
    """
    Get the fallback account ID from accounts.toml if configured.

    Fallback account must have resume_on_failure=true.
    """
    try:
        config = get_accounts_config()
        if "accounts" not in config:
            return None

        for account_id, account_cfg in config["accounts"].items():
            if account_id != ACCOUNT_ID:  # Must be different from current
                if account_cfg.get("resume_on_failure", False):
                    return account_id

        return None
    except Exception as e:
        logger.error(f"Error reading fallback account: {e}")
        return None


def layer3_switch_account(fallback_account: str) -> bool:
    """
    Layer 3: Switch to fallback account by calling toggle-x-account.sh.

    Returns True if switch succeeded, False otherwise.
    """
    global LAYER3_RETRY_COUNT

    LAYER3_RETRY_COUNT += 1
    logger.info(f"[Layer 3] Switching to fallback account: {fallback_account}")

    try:
        # Look for toggle script in scripts/hermes-x/ (same parent as core/)
        script_path = Path(__file__).parent.parent / "toggle-x-account.sh"
        if not script_path.exists():
            logger.error(f"Toggle script not found: {script_path}")
            return False

        result = subprocess.run(
            ["bash", str(script_path), fallback_account],
            timeout=30,
            capture_output=True,
            text=True,
        )

        if result.returncode == 0:
            logger.info(f"[Layer 3] Successfully switched to {fallback_account}")
            return True
        else:
            logger.error(f"[Layer 3] Switch failed: {result.stderr}")
            return False

    except subprocess.TimeoutExpired:
        logger.error(f"[Layer 3] Switch timeout")
        return False
    except Exception as e:
        logger.error(f"[Layer 3] Error switching account: {e}")
        return False


def layer4_alert_kernel():
    """
    Layer 4: Post critical alert to kernel because all recovery attempts failed.
    """
    logger.critical(f"[Layer 4] All recovery attempts failed for {ACCOUNT_ID}. Alerting kernel.")

    try:
        import requests

        headers = {}
        if API_KEY:
            headers["Authorization"] = f"Bearer {API_KEY}"
        headers["Content-Type"] = "application/json"

        payload = {
            "tool": "hermes-x-recovery",
            "domain": "organ-health",
            "target": ACCOUNT_ID,
            "status": "critical",
            "context": f"account={ACCOUNT_ID} layer2_retries={LAYER2_RETRY_COUNT} layer3_retries={LAYER3_RETRY_COUNT} reason=x_auth_expired manual_intervention_required=true",
            "consumer": f"human@{ACCOUNT_ID}",
            "agent_id": f"hermes-x-recovery-{ACCOUNT_ID}",
            "tags": ["recovery-failed", f"account-{ACCOUNT_ID}"],
        }

        url = f"{REST_ADDR}/observe"
        resp = requests.post(url, json=payload, headers=headers, timeout=10)

        if resp.status_code == 200:
            logger.info("[Layer 4] Critical alert sent to kernel")
        else:
            logger.error(f"[Layer 4] Failed to alert kernel: {resp.status_code}")

    except Exception as e:
        logger.error(f"[Layer 4] Error alerting kernel: {e}")


def run_recovery_cycle():
    """Main recovery loop — check heartbeat and handle failures."""
    global FAILURE_COUNT, LAYER2_RETRY_COUNT, LAYER3_RETRY_COUNT

    heartbeat = check_heartbeat()

    if not heartbeat:
        logger.debug("No heartbeat data available")
        return

    # Check if heartbeat reports auth failure
    if heartbeat.get("status") == "critical" and heartbeat.get("failure_reason") == "x_auth_expired":
        FAILURE_COUNT += 1
        logger.warning(f"Auth failure detected ({FAILURE_COUNT}/{MAX_FAILURE_BEFORE_LAYER2})")

        # Layer 1 + 2: Detect auth failure, then retry login
        if FAILURE_COUNT >= MAX_FAILURE_BEFORE_LAYER2 and LAYER2_RETRY_COUNT == 0:
            if layer2_retry_login():
                # Login succeeded — reset counters
                logger.info("Recovery successful at Layer 2. Resetting failure counters.")
                FAILURE_COUNT = 0
                LAYER2_RETRY_COUNT = 0
                return

            # Layer 2 retry failed, check if we should attempt Layer 3
            if LAYER2_RETRY_COUNT >= MAX_LAYER2_RETRIES:
                fallback = get_fallback_account()
                if fallback:
                    if layer3_switch_account(fallback):
                        logger.info("Successfully switched to fallback account.")
                        FAILURE_COUNT = 0
                        LAYER2_RETRY_COUNT = 0
                        LAYER3_RETRY_COUNT = 0
                        return
                    # Fallback failed — escalate to Layer 4
                else:
                    logger.warning("No fallback account configured. Escalating to Layer 4.")

                # All recovery attempts exhausted
                layer4_alert_kernel()
                # Don't reset counters — keep in failed state until manual intervention

    else:
        # Heartbeat is healthy — reset all counters
        if FAILURE_COUNT > 0:
            logger.info("Heartbeat healthy. Resetting failure counters.")
        FAILURE_COUNT = 0
        LAYER2_RETRY_COUNT = 0
        LAYER3_RETRY_COUNT = 0


def main():
    """Main entry point."""
    logger.info(f"Hermes X Recovery Daemon started for account: {ACCOUNT_ID}")
    logger.info(f"Kernel: {REST_ADDR}")
    logger.info(f"Check interval: {CHECK_INTERVAL}s")

    # Validate account configuration (Phase 2a.3 — fail-fast on misconfiguration)
    # Use interactive=False for systemd mode — no TTY available, fail cleanly if credentials missing
    try:
        username, _ = get_x_credentials(interactive=False)  # Discard password; validation only
        logger.info(f"Account validation passed: {username}")
    except RuntimeError as e:
        logger.error(f"Account configuration invalid: {e}")
        sys.exit(1)

    try:
        while True:
            try:
                run_recovery_cycle()
            except Exception as e:
                logger.error(f"Error in recovery cycle: {e}", exc_info=True)

            time.sleep(CHECK_INTERVAL)

    except KeyboardInterrupt:
        logger.info("Recovery daemon stopped by user")
        sys.exit(0)
    except Exception as e:
        logger.critical(f"Unrecoverable error: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    main()
