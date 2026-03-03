import subprocess
from pathlib import Path


def run_cmd(cmd):
    try:
        result = subprocess.run(cmd, capture_output=True, text=True)
        return result.returncode == 0, result.stdout, result.stderr
    except Exception as e:
        return False, "", str(e)


def master_heal():
    print("--- 🛡️  CYNIC MASTER HEALER : INITIATING PURIFICATION ---")

    # 1. Identify all Python files in cynic/
    cynic_dir = Path("cynic")
    py_files = list(cynic_dir.rglob("*.py"))

    total = len(py_files)
    healed = 0
    failed = 0

    for i, file_path in enumerate(py_files):
        rel_path = str(file_path)
        print(f"[{i+1}/{total}] Auditing: {rel_path}...")

        # Check if file has syntax errors
        ok, _, err = run_cmd(["ruff", "check", rel_path, "--select", "E999"])

        if not ok:
            print(f"  ☢️  Anomaly detected in {rel_path}. Attempting surgery...")

            # Step A: Ruff auto-fix
            run_cmd(["ruff", "check", rel_path, "--fix"])
            run_cmd(["ruff", "format", rel_path])

            # Step B: Verify with HeresyGuard
            success, _, _ = run_cmd(["python", "scripts/verify_surgery.py", rel_path])

            if success:
                print(f"  ✅ {rel_path} HEALED.")
                healed += 1
            else:
                # If still failing, it requires manual intervention later
                print(f"  ❌ {rel_path} remains FRAGILE. Manual surgery needed.")
                failed += 1
        else:
            # print(f"  💎 {rel_path} is stable.")
            pass

    print("\n" + "=" * 40)
    print("PURIFICATION COMPLETE.")
    print(f"Healed: {healed} | Failed: {failed} | Stable: {total - healed - failed}")
    print("=" * 40)


if __name__ == "__main__":
    master_heal()
