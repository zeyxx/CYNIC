#!/usr/bin/env python3
"""Verify new modules import correctly"""
import sys
import os

# UTF-8 output on Windows
if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

print("\n" + "="*70)
print("SANITY CHECK: Module Imports")
print("="*70)

errors = []

# Test 1: EnvironmentIntrospector
print("\n[1/2] Testing EnvironmentIntrospector...")
try:
    from cynic.perceive.environment import EnvironmentIntrospector
    print("      OK - Imported successfully")
except Exception as e:
    print(f"      ERROR: {e}")
    errors.append(("EnvironmentIntrospector", str(e)))

# Test 2: ConfigurationAdaptationEngine
print("[2/2] Testing ConfigurationAdaptationEngine...")
try:
    from cynic.core.config_adapter import ConfigurationAdaptationEngine
    print("      OK - Imported successfully")
except Exception as e:
    print(f"      ERROR: {e}")
    errors.append(("ConfigurationAdaptationEngine", str(e)))

# Summary
print("\n" + "="*70)
if not errors:
    print("RESULT: All modules importable")
    print("="*70)
    sys.exit(0)
else:
    print("RESULT: Import errors detected")
    for name, error in errors:
        print(f"  - {name}: {error}")
    print("="*70)
    sys.exit(1)
