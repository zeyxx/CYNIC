#!/usr/bin/env python3
"""
CYNIC ACTIVATION VERIFICATION SCRIPT
Comprehensive check that all components are working end-to-end
"""

import asyncio
import json
import subprocess
import sys
from pathlib import Path

# Colors for terminal output
class Colors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

def print_section(title):
    print(f"\n{Colors.HEADER}{Colors.BOLD}{'='*70}")
    print(f"{title:^70}")
    print(f"{'='*70}{Colors.ENDC}\n")

def print_ok(msg):
    print(f"{Colors.OKGREEN}✅ {msg}{Colors.ENDC}")

def print_warn(msg):
    print(f"{Colors.WARNING}⚠️  {msg}{Colors.ENDC}")

def print_fail(msg):
    print(f"{Colors.FAIL}❌ {msg}{Colors.ENDC}")

def print_info(msg):
    print(f"{Colors.OKCYAN}ℹ️  {msg}{Colors.ENDC}")

def run_cmd(cmd):
    """Run shell command and return output"""
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=10)
        return result.stdout.strip(), result.stderr.strip(), result.returncode
    except subprocess.TimeoutExpired:
        return "", "Timeout", 1
    except Exception as e:
        return "", str(e), 1

async def verify_docker():
    """Check Docker containers are running"""
    print_section("1. DOCKER CONTAINERS STATUS")

    stdout, stderr, rc = run_cmd("docker-compose ps --format json")

    if rc != 0:
        print_fail(f"docker-compose ps failed: {stderr}")
        return False

    try:
        containers = json.loads(stdout)
        running = {c['Service']: c['State'] for c in containers}

        required = ['postgres-py', 'ollama', 'cynic']
        all_ok = True

        for svc in required:
            state = running.get(svc, 'UNKNOWN')
            if 'running' in state.lower() or 'up' in state.lower():
                print_ok(f"{svc}: {state}")
            else:
                print_fail(f"{svc}: {state}")
                all_ok = False

        return all_ok
    except json.JSONDecodeError:
        print_fail("Failed to parse docker-compose output")
        return False

async def verify_env():
    """Check .env file configuration"""
    print_section("2. ENVIRONMENT CONFIGURATION")

    env_file = Path("/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC/.env")

    if not env_file.exists():
        print_fail(".env file not found")
        return False

    with open(env_file) as f:
        content = f.read()

    checks = {
        'CUSTOM_MODELS_PATH': 'D:\\Models' in content,
        'CYNIC_MODELS_DIR': '/models' in content,
        'LLAMA_CPP_GPU_LAYERS': 'LLAMA_CPP_GPU_LAYERS' in content,
        'LOG_LEVEL=DEBUG': 'LOG_LEVEL=DEBUG' in content,
    }

    all_ok = True
    for key, found in checks.items():
        if found:
            print_ok(f"{key} configured")
        else:
            print_warn(f"{key} not found")
            all_ok = False

    return all_ok

async def verify_volumes():
    """Check Docker volumes and mounts"""
    print_section("3. DOCKER VOLUME MOUNTS")

    # Check if /models is accessible in cynic container
    stdout, stderr, rc = run_cmd("docker-compose exec cynic ls -la /models 2>&1 | head -10")

    if rc == 0 and stdout:
        print_ok("Custom models volume /models mounted in container")
        print_info(stdout[:200])
        return True
    else:
        print_warn("Custom models volume not found or empty")
        print_info(f"stderr: {stderr[:200]}")
        return False

async def verify_llm_discovery():
    """Check if LLM models were discovered"""
    print_section("4. LLM MODEL DISCOVERY")

    # Query the consciousness endpoint for LLM status
    stdout, stderr, rc = run_cmd(
        "curl -s http://localhost:8000/consciousness 2>&1 | "
        "python3 -c \"import json, sys; d=json.load(sys.stdin); "
        "print(json.dumps(d.get('mirror', {}).get('sage', {}), indent=2))\" 2>&1"
    )

    if rc == 0:
        print_ok("SAGE status retrieved")
        print_info(stdout[:300])

        # Check for llm_count
        if 'llm_count' in stdout and int(stdout.split('llm_count')[1].split(':')[1].split(',')[0].strip()) > 0:
            print_ok("LLM calls detected!")
            return True
        else:
            print_warn("No LLM calls yet (might still be discovering)")
            return False
    else:
        print_fail(f"Failed to query consciousness: {stderr[:200]}")
        return False

async def verify_macro_judgment():
    """Test MACRO consciousness with LLM"""
    print_section("5. MACRO JUDGMENT TEST (End-to-End)")

    payload = {
        "subject": "Test CYNIC LLM activation",
        "code": "def hello():\n    return 'world'",
        "level": "MACRO"
    }

    import json as json_module
    payload_json = json_module.dumps(payload)

    stdout, stderr, rc = run_cmd(
        f"curl -s -X POST http://localhost:8000/judge "
        f"-H 'Content-Type: application/json' "
        f"-d '{payload_json}' 2>&1"
    )

    if rc == 0 and stdout:
        try:
            result = json_module.loads(stdout)

            # Check for key indicators
            q_score = result.get('q_score')
            llm_calls = result.get('llm_calls', 0)
            verdict = result.get('verdict')

            print_ok(f"Judgment returned: Q={q_score}, verdict={verdict}")

            if llm_calls > 0:
                print_ok(f"✅ LLM INFERENCE ACTIVE! {llm_calls} LLM calls made")
                return True
            else:
                print_warn(f"Q={q_score} but llm_calls={llm_calls} (still using heuristics)")
                return False
        except json_module.JSONDecodeError:
            print_fail(f"Invalid JSON response: {stdout[:200]}")
            return False
    else:
        print_fail(f"Judgment request failed: {stderr[:200]}")
        return False

async def verify_dogs():
    """Check all 11 dogs are reporting"""
    print_section("6. DOG REPORTING STATUS")

    stdout, stderr, rc = run_cmd(
        "curl -s http://localhost:8000/consciousness 2>&1 | "
        "python3 -c \"import json, sys; d=json.load(sys.stdin); "
        "dogs = d.get('mirror', {}).get('dogs', {}); "
        "print('\\n'.join([f'{k}: {v.get(\\\"judgment_count\\\", 0)}' for k, v in dogs.items()]))\" 2>&1"
    )

    if rc == 0 and stdout:
        lines = stdout.strip().split('\n')
        active_dogs = [l for l in lines if ':' in l]
        print_ok(f"Dogs reporting: {len(active_dogs)}/11")

        for line in active_dogs[:11]:
            parts = line.split(': ')
            if len(parts) == 2:
                dog_name, count = parts
                if int(count) > 0:
                    print_ok(f"  {dog_name}: {count} judgments")
                else:
                    print_warn(f"  {dog_name}: {count} judgments (dormant)")

        return len(active_dogs) >= 7
    else:
        print_fail(f"Dog status query failed: {stderr[:200]}")
        return False

async def main():
    print(f"\n{Colors.BOLD}{Colors.HEADER}")
    print(r"""
    ╔═══════════════════════════════════════════════════════╗
    ║                                                       ║
    ║     CYNIC ACTIVATION VERIFICATION                    ║
    ║     Making the Organism Real                         ║
    ║                                                       ║
    ║     φ distrusts φ — κυνικός                          ║
    ║                                                       ║
    ╚═══════════════════════════════════════════════════════╝
    """)
    print(Colors.ENDC)

    results = {}

    # Run all verifications
    results['docker'] = await verify_docker()
    results['env'] = await verify_env()
    results['volumes'] = await verify_volumes()
    results['llm_discovery'] = await verify_llm_discovery()
    results['macro_judgment'] = await verify_macro_judgment()
    results['dogs'] = await verify_dogs()

    # Summary
    print_section("ACTIVATION SUMMARY")

    passed = sum(1 for v in results.values() if v)
    total = len(results)

    for check, result in results.items():
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"{status} — {check}")

    print(f"\n{Colors.BOLD}Overall: {passed}/{total} checks passed{Colors.ENDC}")

    if passed == total:
        print(f"\n{Colors.OKGREEN}{Colors.BOLD}")
        print("🎉 CYNIC IS ALIVE AND READY! 🎉")
        print("All systems operational, LLM inference active.")
        print(f"{Colors.ENDC}\n")
        return 0
    elif passed >= 4:
        print(f"\n{Colors.WARNING}{Colors.BOLD}")
        print("⚠️  CYNIC PARTIALLY ACTIVE")
        print("Some systems operational, working on full activation...")
        print(f"{Colors.ENDC}\n")
        return 1
    else:
        print(f"\n{Colors.FAIL}{Colors.BOLD}")
        print("❌ CYNIC NEEDS MORE SETUP")
        print("Check failures above and retry after fixes.")
        print(f"{Colors.ENDC}\n")
        return 2

if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
