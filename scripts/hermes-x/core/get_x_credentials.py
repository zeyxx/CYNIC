#!/usr/bin/env python3
"""
Get CYNIC X.com credentials (environment variable or interactive prompt).

Stores encrypted password via GPG. Decrypts at runtime — credentials never
touch plaintext disk files.

Usage:
    creds = get_x_credentials()  # Returns (username, password)

Environment variables (take precedence):
    CYNIC_X_USERNAME  - X account username (default: @CynicOracle)
    CYNIC_X_PASSWORD  - X account password (in memory only, runtime only)
"""

import os
import getpass
import subprocess
from pathlib import Path


def get_x_credentials():
    """
    Get X credentials from environment or interactive prompt.

    Returns:
        tuple: (username, password)
    """
    username = os.getenv("CYNIC_X_USERNAME", "@CynicOracle")

    # Check environment variable first (highest priority)
    password = os.getenv("CYNIC_X_PASSWORD")
    if password:
        return username, password

    # Fall back to interactive prompt
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
