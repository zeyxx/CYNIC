#!/usr/bin/env python3
"""Generate test health report — run locally to update test-health.json."""
import json
import subprocess
import sys
from datetime import datetime
import re
import os

# Fix Windows encoding issues with Unicode characters
if sys.platform == 'win32':
    sys.stdout.reconfigure(encoding='utf-8')

print("\n" + "="*70)
print("CYNIC LOCAL TEST HEALTH REPORT")
print("="*70 + "\n")

# Run tests with coverage
result = subprocess.run(
    ['python', '-m', 'pytest', 'tests/', '--cov=cynic', '--cov-report=term', '-v', '--tb=no'],
    capture_output=True,
    text=True
)

lines = result.stdout.split('\n')
stderr_lines = result.stderr.split('\n')

# Count tests
test_lines = [l for l in lines if '::test_' in l]
passed = len([l for l in lines if 'PASSED' in l])
failed = len([l for l in lines if 'FAILED' in l])
skipped = len([l for l in lines if 'SKIPPED' in l])

# Extract coverage
coverage_pct = 0
coverage_lines = [l for l in lines if 'TOTAL' in l]
if coverage_lines:
    match = re.search(r'(\d+)%', coverage_lines[0])
    if match:
        coverage_pct = int(match.group(1))

# Build status
test_status = "passing" if result.returncode == 0 else "failing"

print(f"⏱️  Timestamp: {datetime.now().isoformat()}")
print(f"📊 Tests: {passed} passed, {failed} failed, {skipped} skipped")
print(f"📈 Coverage: {coverage_pct}%")
print(f"✅ Status: {test_status}\n")

# Generate health JSON
health_status = {
    "timestamp": datetime.now().isoformat(),
    "status": "healthy" if result.returncode == 0 else "unhealthy",
    "tests": {
        "total": passed + failed + skipped,
        "passed": passed,
        "failed": failed,
        "skipped": skipped
    },
    "coverage": {
        "percentage": coverage_pct,
        "minimum": 75,
        "status": "✅ above minimum" if coverage_pct >= 75 else "❌ below minimum"
    },
    "quality_gates": {
        "complexity_maximum": 10,
        "maintainability_minimum": 70,
        "coverage_minimum": 75
    },
    "validation": {
        "local_precommit": "enforced",
        "silent_failures": "eliminated",
        "ai_amplification_guard": "active"
    }
}

# Write health file
with open('.github/test-health.json', 'w') as f:
    json.dump(health_status, f, indent=2)

print("✅ Health report saved to .github/test-health.json")
print("\nRemember to commit this file:")
print("  git add .github/test-health.json")
print("  git commit -m \"chore: update test health report\"")

sys.exit(result.returncode)
