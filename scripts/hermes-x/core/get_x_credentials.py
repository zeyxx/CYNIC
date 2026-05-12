#!/usr/bin/env python3
"""
Get X.com credentials from accounts.toml or environment variables.

Multi-account aware: reads HERMES_ACCOUNT env var to select account from
~/.config/cynic/accounts.toml. Falls back to CYNIC_X_USERNAME/CYNIC_X_PASSWORD
for backward compatibility.

Usage:
    creds = get_x_credentials()  # Returns (username, password)

Environment variables (in order of precedence):
    HERMES_ACCOUNT        - Account ID (cynic, personal, etc.) — selects account from accounts.toml
    CYNIC_X_USERNAME      - X account username (fallback if no accounts.toml)
    CYNIC_X_PASSWORD      - X account password (fallback, in memory only)
"""

import os
import getpass
import subprocess
import tomllib
from pathlib import Path


def load_accounts_config() -> dict:
    """Load accounts.toml from ~/.config/cynic/."""
    config_path = Path.home() / ".config/cynic/accounts.toml"
    if not config_path.exists():
        return {}
    try:
        with open(config_path, "rb") as f:
            return tomllib.load(f)
    except Exception as e:
        raise RuntimeError(f"Failed to load accounts.toml: {e}")


def get_x_credentials():
    """
    Get X credentials from accounts.toml or environment.

    Priority:
    1. HERMES_ACCOUNT → accounts.toml[accounts.HERMES_ACCOUNT]
    2. CYNIC_X_USERNAME/CYNIC_X_PASSWORD env vars (legacy)
    3. Interactive prompt

    Returns:
        tuple: (username, password)
    """
    account_id = os.getenv("HERMES_ACCOUNT", "").strip()

    # Try accounts.toml first
    if account_id:
        accounts = load_accounts_config()
        if "accounts" in accounts and account_id in accounts["accounts"]:
            account = accounts["accounts"][account_id]
            username = account.get("username", "")
            password_env = account.get("password_env", "")

            if not username:
                raise RuntimeError(f"Account {account_id} has no username in accounts.toml")

            # Get password from env var specified in config
            password = ""
            if password_env:
                password = os.getenv(password_env, "")

            if password:
                return username, password

            # For read-only accounts (like personal with OAuth), prompt for password
            # or fail gracefully if no password available
            print(f"X credentials required for {username} ({account_id})")
            password = getpass.getpass("X.com password (leave blank for OAuth): ")  # HARDCODED_CREDS=safe-prompt
            if password:
                return username, password
            # No password — may be OAuth (personal account)
            return username, ""

    # Fallback to legacy env vars
    username = os.getenv("CYNIC_X_USERNAME", "@CynicOracle")
    password = os.getenv("CYNIC_X_PASSWORD")
    if password:
        return username, password

    # Interactive prompt (legacy)
    print(f"X credentials required for {username}")
    password = getpass.getpass("X.com password: ")

    if not password:
        raise RuntimeError("X password is required")

    return username, password


def store_x_password_encrypted(password: str):
    """
    Store password encrypted via GPG (optional, for manual setup).

    Example:
        store_x_password_encrypted(getpass.getpass())
        # Then read via: gpg --decrypt ~/.cynic/organs/hermes/x/.x_password.gpg
    """
    creds_dir = Path.home() / ".cynic/organs/hermes/x"
    creds_dir.mkdir(parents=True, exist_ok=True)

    password_file = creds_dir / ".x_password.gpg"

    # Encrypt password via GPG
    proc = subprocess.run(
        ["gpg", "--batch", "--trust-model", "always", "--encrypt", "--armor"],
        input=password.encode(),
        capture_output=True,
    )

    if proc.returncode != 0:
        raise RuntimeError(f"GPG encryption failed: {proc.stderr.decode()}")

    with open(password_file, "wb") as f:
        f.write(proc.stdout)

    print(f"Password encrypted and stored at: {password_file}")
    print(f"To decrypt: gpg --decrypt {password_file}")


if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1 and sys.argv[1] == "store":
        # Manual setup: python3 get_x_credentials.py store
        password = getpass.getpass("X.com password to store (encrypted): ")
        store_x_password_encrypted(password)
    else:
        # Retrieve credentials
        username, password = get_x_credentials()
        print(f"Username: {username}")
        print(f"Password loaded (length: {len(password)} chars)")
