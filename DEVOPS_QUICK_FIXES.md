# CYNIC DevOps Quick Fixes

## CRITICAL: Dockerfile Fix (1 line change)

**File:** `Dockerfile` line 22

**Change:**
```bash
# FROM:
RUN mkdir -p /home/cynic/.cynic && chown -r cynic:cynic /home/cynic

# TO:
RUN mkdir -p /home/cynic/.cynic && chown -R cynic:cynic /home/cynic
```

**Reason:** `chown` flag is `-R` (uppercase), not `-r` (lowercase)

**Verification:**
```bash
docker build -t cynic:test .
docker run --rm cynic:test python -c "import cynic; print('OK')"
```

---

## GitHub Actions YAML Fixes

### File 1: `.github/workflows/ci-gates.yml` (line ~1644)
Issue: Multiline string with special characters

Review and ensure all YAML multiline strings use proper quoting.

### File 2: `.github/workflows/documentation.yml` (line ~114)
Issue: Python code block in YAML

Wrap Python code in proper YAML string format (use `|` or `|-`).

### File 3: `.github/workflows/release.yml` (line ~117)
Issue: Heredoc with special characters

Wrap heredoc in proper YAML quoting.

### File 4: `.github/workflows/pr-validation.yml`
Issue: Encoding with special characters

Ensure UTF-8 encoding or escape problematic characters.

**Verification:**
```bash
python3 -c "
import yaml
files = ['.github/workflows/ci-gates.yml', ...]
for f in files:
    try:
        with open(f, 'r', encoding='utf-8') as fp:
            yaml.safe_load(fp)
            print(f'{f}: OK')
    except Exception as e:
        print(f'{f}: ERROR - {e}')
"
```

---

## Optional: Add YAML Validation to Pre-Commit

**File:** `.git/hooks/pre-commit` (add new gate)

```bash
# Gate X: YAML Validation
echo "  [X/5] Validating GitHub Actions YAML..."
python3 << 'EOF'
import yaml
import sys
from pathlib import Path

errors = []
for f in Path('.github/workflows').glob('*.yml'):
    try:
        with open(f, 'r', encoding='utf-8') as fp:
            yaml.safe_load(fp)
    except Exception as e:
        errors.append(f"{f}: {e}")

if errors:
    for err in errors:
        print(f"ERROR: {err}")
    sys.exit(1)
print("OK: All YAML files valid")
EOF
|| exit 1
```

---

## Test the Fixes

```bash
# 1. Build Docker image
docker build -t cynic:test .

# 2. Run smoke test
docker run --rm cynic:test python -c "import cynic; print('Docker build OK')"

# 3. Validate GitHub Actions
python3 -m pip install pyyaml
python3 scripts/validate_yaml_workflows.py

# 4. Run full pre-commit suite
git add .
git commit -m "Fix: DevOps infrastructure (Dockerfile, GitHub Actions YAML)"
```

---

## Deployment Readiness Checklist

- [ ] Dockerfile builds successfully
- [ ] Docker-compose services start
- [ ] All GitHub Actions YAML files parse
- [ ] All validation scripts pass
- [ ] 1204 tests pass
- [ ] Health endpoints respond
- [ ] Metrics endpoint works (/metrics)
- [ ] CI/CD pipeline executes without errors

---

## Timeline

- **Immediate (5 min):** Fix Dockerfile `-r` to `-R`
- **Short-term (20 min):** Fix GitHub Actions YAML syntax
- **Validation (10 min):** Run test suite and health checks
- **Total:** ~35 minutes to production-ready

---

## What's Already Working

- Prometheus metrics integration (7 metrics)
- Health check endpoints (5 endpoints)
- Docker-compose services (4 services)
- Validation scripts (4 scripts, all passing)
- Pre-commit hooks (5 gates)
- Test suite (1204 tests)
- Logging infrastructure (30+ modules)
- Factory wiring (verified)
- API routers (27 mounted)

**No rebuild needed for these components.**

---

## What Needs New Development

- `release.sh` - Automated release pipeline
- `rollback.sh` - Emergency rollback script
- Kubernetes manifests - If K8s deployment needed
- Terraform modules - If cloud deployment needed
- Blue-green deployment - For zero-downtime updates

**These are optional for MVP, required for production at scale.**
