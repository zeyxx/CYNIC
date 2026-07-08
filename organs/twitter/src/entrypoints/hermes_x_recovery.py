#!/usr/bin/env python3
"""Hermes X recovery daemon entrypoint."""

from __future__ import annotations

from organs.twitter.src.adapters import hermes_x_recovery_runtime as runtime


subprocess = runtime.subprocess
ACCOUNT_ID = runtime.ACCOUNT_ID
REST_ADDR = runtime.REST_ADDR
API_KEY = runtime.API_KEY
CHECK_INTERVAL = runtime.CHECK_INTERVAL
FAILURE_COUNT = runtime.FAILURE_COUNT
LAYER2_RETRY_COUNT = runtime.LAYER2_RETRY_COUNT
LAYER3_RETRY_COUNT = runtime.LAYER3_RETRY_COUNT
MAX_LAYER2_RETRIES = runtime.MAX_LAYER2_RETRIES
MAX_FAILURE_BEFORE_LAYER2 = runtime.MAX_FAILURE_BEFORE_LAYER2
logger = runtime.logger


def _sync_to_runtime() -> None:
    runtime.subprocess = subprocess
    runtime.ACCOUNT_ID = ACCOUNT_ID
    runtime.REST_ADDR = REST_ADDR
    runtime.API_KEY = API_KEY
    runtime.CHECK_INTERVAL = CHECK_INTERVAL
    runtime.FAILURE_COUNT = FAILURE_COUNT
    runtime.LAYER2_RETRY_COUNT = LAYER2_RETRY_COUNT
    runtime.LAYER3_RETRY_COUNT = LAYER3_RETRY_COUNT


def _sync_from_runtime() -> None:
    globals()["FAILURE_COUNT"] = runtime.FAILURE_COUNT
    globals()["LAYER2_RETRY_COUNT"] = runtime.LAYER2_RETRY_COUNT
    globals()["LAYER3_RETRY_COUNT"] = runtime.LAYER3_RETRY_COUNT


def _normalize_kernel_addr(raw: str) -> str:
    return runtime._normalize_kernel_addr(raw)


def check_heartbeat():
    return runtime.check_heartbeat()


def layer2_retry_login() -> bool:
    _sync_to_runtime()
    result = runtime.layer2_retry_login()
    _sync_from_runtime()
    return result


def layer3_switch_account(fallback_account: str) -> bool:
    _sync_to_runtime()
    result = runtime.layer3_switch_account(fallback_account)
    _sync_from_runtime()
    return result


def layer4_alert_kernel():
    _sync_to_runtime()
    return runtime.layer4_alert_kernel()


def run_recovery_cycle():
    _sync_to_runtime()
    result = runtime.run_recovery_cycle()
    _sync_from_runtime()
    return result


def main():
    _sync_to_runtime()
    return runtime.main()


if __name__ == "__main__":
    main()
