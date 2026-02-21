#!/usr/bin/env python3
"""Phase 0: Move ~/.cynic to D:\cynic_state"""
import shutil
import os
from pathlib import Path
import sys

# Set UTF-8 output on Windows
if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

print("\n" + "="*70)
print("PHASE 0: Move ~/.cynic to D:\\ (Consolidation Preparation)")
print("="*70)

# Paths
cynic_home = Path.home() / ".cynic"
d_cynic = Path("D:/cynic_state")

print(f"\n[1/6] Paths:")
print(f"      Source: {cynic_home}")
print(f"      Dest:   {d_cynic}")

# Create destination
print(f"\n[2/6] Creating destination directory...")
try:
    d_cynic.mkdir(parents=True, exist_ok=True)
    print(f"      OK - {d_cynic}")
except Exception as e:
    print(f"      ERROR: {e}")
    sys.exit(1)

# Check source
print(f"\n[3/6] Analyzing source...")
if cynic_home.exists():
    if cynic_home.is_symlink():
        print(f"      INFO - ~/.cynic is already a symlink pointing to:")
        print(f"             {cynic_home.resolve()}")
        sys.exit(0)

    files = [f for f in cynic_home.rglob("*") if f.is_file()]
    total_size = sum(f.stat().st_size for f in files) / 1024 / 1024
    print(f"      Files: {len(files)}")
    print(f"      Size:  {total_size:.1f} MB")
else:
    print(f"      INFO - ~/.cynic doesn't exist yet")
    print(f"             Will be created on first CYNIC run")
    sys.exit(0)

# Copy files
print(f"\n[4/6] Copying to D:\\cynic_state...")
try:
    for src_file in files:
        rel_path = src_file.relative_to(cynic_home)
        dest_file = d_cynic / rel_path
        dest_file.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src_file, dest_file)
    print(f"      OK - Copied {len(files)} files")
except Exception as e:
    print(f"      ERROR: {e}")
    sys.exit(1)

# Backup original
print(f"\n[5/6] Backing up original...")
backup = Path.home() / ".cynic.backup"
try:
    if backup.exists():
        shutil.rmtree(backup)
    shutil.move(str(cynic_home), str(backup))
    print(f"      OK - Backed up to {backup}")
except Exception as e:
    print(f"      ERROR: {e}")
    sys.exit(1)

# Instructions for symlink
print(f"\n[6/6] Create symlink (manual step)...")
print(f"      Run this command as Administrator in PowerShell:")
print(f"      ")
print(f"      cmd /c mklink /D \"%USERPROFILE%\\.cynic\" \"D:\\cynic_state\"")
print(f"      ")
print(f"      OR use: python -m cynic.cli setup-symlink")

# Summary
print(f"\n" + "="*70)
print(f"PHASE 0 COMPLETE")
print(f"="*70)
print(f"\nDisk freed on C:  ~{total_size:.1f} MB")
print(f"Backup location: {backup}")
print(f"New location:    D:\\cynic_state")
print(f"\nNext step: Update .env with new paths")
print()
