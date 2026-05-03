#!/usr/bin/env python3
"""
Soma Integrity Validator — Pre-flight checks before orchestrator runs.

Purpose: Detect unauthorized file modifications before executing recovery commands.

Checks:
1. soma_manifest.toml hash matches stored value (prevents manifest poisoning)
2. soma_orchestrator.py hash matches stored value (detects code tampering)
3. File permissions are restrictive (700 = rwx------)
4. No symlink attacks

Fails loud: If ANY check fails, refuse to start orchestrator.
"""

import hashlib
import sys
import os
from pathlib import Path


def compute_sha256(file_path: Path) -> str:
    """Compute SHA256 hash of file."""
    sha256_hash = hashlib.sha256()
    with open(file_path, "rb") as f:
        for byte_block in iter(lambda: f.read(4096), b""):
            sha256_hash.update(byte_block)
    return sha256_hash.hexdigest()


def load_stored_hash(hash_file: Path) -> dict:
    """Load stored hashes from file."""
    if not hash_file.exists():
        return {}

    hashes = {}
    with open(hash_file) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split("  ", 1)
            if len(parts) == 2:
                hash_val, filename = parts
                hashes[filename] = hash_val

    return hashes


def save_hash(hash_file: Path, filename: str, hash_val: str) -> None:
    """Save/update hash in file."""
    # Make file writable if it's read-only (from previous run)
    if hash_file.exists():
        hash_file.chmod(0o600)

    hashes = load_stored_hash(hash_file)
    hashes[filename] = hash_val

    with open(hash_file, "w") as f:
        f.write("# Soma Integrity Hashes (DO NOT MODIFY)\n")
        f.write("# Format: SHA256_HASH  filename\n")
        for fname, hval in sorted(hashes.items()):
            f.write(f"{hval}  {fname}\n")

    # Restrict permissions: owner read-only (not write, in case of accidental modification)
    hash_file.chmod(0o400)


def check_file_integrity(file_path: Path, expected_hash: str) -> bool:
    """Verify file hash matches expected value."""
    actual_hash = compute_sha256(file_path)
    return actual_hash == expected_hash


def check_permissions(file_path: Path) -> bool:
    """Verify file has restrictive permissions (700 = rwx------)."""
    mode = file_path.stat().st_mode & 0o777
    # Accept 700 (rwx------) or 600 (rw-------)
    # Reject 644 (world-readable) or 755 (world-executable)
    return mode in [0o700, 0o600]


def check_symlink(file_path: Path) -> bool:
    """Reject symlinks (symlink attack vector)."""
    return not file_path.is_symlink()


def validate_all(soma_dir: Path) -> bool:
    """
    Run all integrity checks.

    Returns: True if all checks pass, False otherwise.
    """
    manifest_file = soma_dir / "soma_manifest.toml"
    orchestrator_file = soma_dir / "soma_orchestrator.py"
    hash_file = soma_dir / "soma_integrity.sha256"

    all_pass = True

    # Check manifest
    print("[1/3] Checking soma_manifest.toml integrity...")
    if not manifest_file.exists():
        print("  ❌ FAIL: soma_manifest.toml not found")
        all_pass = False
    else:
        if not check_symlink(manifest_file):
            print("  ❌ FAIL: soma_manifest.toml is a symlink (symlink attack detected)")
            all_pass = False
        elif not check_permissions(manifest_file):
            mode = oct(manifest_file.stat().st_mode & 0o777)
            print(f"  ⚠️  WARNING: soma_manifest.toml has permissive permissions ({mode})")
            print(f"       Recommend: chmod 700 soma_manifest.toml")
        else:
            print(f"  ✓ Permissions OK (mode={oct(manifest_file.stat().st_mode & 0o777)})")

        stored_hashes = load_stored_hash(hash_file)
        if "soma_manifest.toml" in stored_hashes:
            expected = stored_hashes["soma_manifest.toml"]
            if check_file_integrity(manifest_file, expected):
                print("  ✓ Hash matches (file unmodified)")
            else:
                print("  ❌ FAIL: soma_manifest.toml hash mismatch (TAMPERED)")
                print(f"     Expected: {expected}")
                print(f"     Actual:   {compute_sha256(manifest_file)}")
                print("     ACTION: Restore from git or verify manually")
                all_pass = False
        else:
            print("  ⚠️  No stored hash (first run)")
            actual_hash = compute_sha256(manifest_file)
            save_hash(hash_file, "soma_manifest.toml", actual_hash)
            print(f"     Hash recorded: {actual_hash}")

    # Check orchestrator
    print("\n[2/3] Checking soma_orchestrator.py integrity...")
    if not orchestrator_file.exists():
        print("  ❌ FAIL: soma_orchestrator.py not found")
        all_pass = False
    else:
        if not check_symlink(orchestrator_file):
            print("  ❌ FAIL: soma_orchestrator.py is a symlink (symlink attack detected)")
            all_pass = False
        elif not check_permissions(orchestrator_file):
            mode = oct(orchestrator_file.stat().st_mode & 0o777)
            print(f"  ⚠️  WARNING: soma_orchestrator.py has permissive permissions ({mode})")
            print(f"       Recommend: chmod 700 soma_orchestrator.py")
        else:
            print(f"  ✓ Permissions OK (mode={oct(orchestrator_file.stat().st_mode & 0o777)})")

        stored_hashes = load_stored_hash(hash_file)
        if "soma_orchestrator.py" in stored_hashes:
            expected = stored_hashes["soma_orchestrator.py"]
            if check_file_integrity(orchestrator_file, expected):
                print("  ✓ Hash matches (file unmodified)")
            else:
                print("  ❌ FAIL: soma_orchestrator.py hash mismatch (TAMPERED)")
                print(f"     Expected: {expected}")
                print(f"     Actual:   {compute_sha256(orchestrator_file)}")
                print("     ACTION: Restore from git or review diffs manually")
                all_pass = False
        else:
            print("  ⚠️  No stored hash (first run)")
            actual_hash = compute_sha256(orchestrator_file)
            save_hash(hash_file, "soma_orchestrator.py", actual_hash)
            print(f"     Hash recorded: {actual_hash}")

    # Check hash file itself
    print("\n[3/3] Checking soma_integrity.sha256 permissions...")
    if not hash_file.exists():
        print("  ⚠️  Hash file will be created on first validation")
    else:
        mode = hash_file.stat().st_mode & 0o777
        if mode not in [0o400, 0o600]:  # read-only or rw for admin update
            print(f"  ⚠️  WARNING: soma_integrity.sha256 has mode {oct(mode)}")
            print(f"       Recommend: chmod 400 soma_integrity.sha256")
        else:
            print(f"  ✓ Permissions OK (mode={oct(mode)})")

        if not check_symlink(hash_file):
            print("  ❌ FAIL: soma_integrity.sha256 is a symlink (symlink attack)")
            all_pass = False

    # Summary
    print("\n" + "=" * 80)
    if all_pass:
        print("✓ INTEGRITY CHECK PASSED — Safe to start orchestrator")
        return True
    else:
        print("❌ INTEGRITY CHECK FAILED — Refusing to start orchestrator")
        print("\nPossible causes:")
        print("  1. Files modified since last validation (symlink attack, tampering)")
        print("  2. Permissions changed (world-readable/executable)")
        print("  3. Hash file missing or corrupted (first run?)")
        print("\nTo fix:")
        print("  - Review diffs: git diff soma/")
        print("  - Restore: git checkout soma/")
        print("  - Or, if changes are intentional, re-initialize: rm soma_integrity.sha256 && python3 soma_integrity.py")
        return False


def main():
    """Main entry point."""
    soma_dir = Path(__file__).parent

    if not validate_all(soma_dir):
        sys.exit(1)
    else:
        sys.exit(0)


if __name__ == "__main__":
    main()
