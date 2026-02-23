# Deployment Debt — CYNIC Infrastructure

> Documented during Foundation Cleanup (2026-02-23)
> Confidence: 58% (φ-bounded, audit findings)
> Last Updated: 2026-02-23

---

## Overview

This document captures infrastructure, DevOps, and deployment-level technical debt. The foundation cleanup identified gaps in CI/CD, observability, scaling, and operational readiness.

**Total Debt Estimate**: 40-60 hours to reach production-ready infrastructure.

---

## Deployment Issues

### BLOCKER (Must fix before production)

#### DEPLOY-T1-001: No CI/CD Pipeline

- **Title**: Tests and linting not enforced before merge to main
- **Severity**: BLOCKER
- **Impact**: Broken code merges to main regularly (happened multiple times in Sessions 1-14). Production deployments fail. No quality gates.
- **Effort**: 6 hours
- **Root Cause**: GitHub Actions workflow exists but incomplete. No pre-commit hooks. Developers don't run tests locally.
- **Evidence**:
  - Session 1 merged broken imports (missing `import asyncpg`)
  - Session 8 discovered tests using old code paths while production used new paths
  - No automated catch mechanism
- **Action**:
  1. Create `.github/workflows/test.yml`:
     - Trigger: push to main, PRs
     - Steps: python 3.13, install deps, run pytest, mypy, ruff
     - Fail if: any test fails, mypy errors, ruff violations
  2. Create `.pre-commit-config.yaml`:
     - Hooks: black (format), ruff (lint), mypy (types), pytest (tests)
     - Run on: commit, push
     - Blocks merge if fails
  3. Add branch protection rule: require status checks pass before merge
  4. Document: `.github/CONTRIBUTING.md` — how to set up pre-commit
  5. Target: 0 broken merges; all PRs require passing CI

**Status**: Not started. Critical for Phase 2.

---

#### DEPLOY-T1-002: Environment Dependencies Not Pinned

- **Title**: Python packages not versioned; reproducibility broken
- **Severity**: BLOCKER
- **Impact**: Works on one machine, breaks on another. New installs fail silently. No dependency lockfile.
- **Effort**: 2 hours
- **Root Cause**: pyproject.toml exists but dependencies use loose version specs (e.g., `asyncpg>=0.27.0` not `asyncpg==0.29.0`). No lock file generated.
- **Evidence**:
  - pyproject.toml shows version ranges
  - New setup may pull incompatible patch versions
  - No poetry.lock or requirements.lock in repo
- **Action**:
  1. Generate lock file: `pip install pip-tools && pip-compile pyproject.toml` (or `poetry lock`)
  2. Pin all direct dependencies: asyncpg==0.29.0 (specific version)
  3. Pin all transitive dependencies: included in lock file
  4. Commit lock file to repo
  5. Update CI: use lock file for installs (`pip install -r requirements.txt`)
  6. Document optional dependencies in README (docker, etc.)
  7. Target: Reproducible environments across all machines

**Status**: pyproject.toml exists, lock file not committed.

---

#### DEPLOY-T1-003: Docker Image Not Production-Ready

- **Title**: Dockerfile and docker-compose lack security, scaling, monitoring
- **Severity**: BLOCKER
- **Impact**: Can't deploy to production. No health checks, no logging, no resource limits. Container crashes silently.
- **Effort**: 8 hours
- **Root Cause**: Task 6 (2026-02-23) created basic docker-compose, but lacks production hardening.
- **Evidence**:
  - Session 6 created docker-compose.yml with defaults
  - No logging configuration
  - No health checks
  - No resource limits
  - No security scanning
- **Action**:
  1. Enhance Dockerfile:
     - Use multi-stage build (builder, runtime) to reduce image size
     - Add non-root user: `RUN useradd -m cynic`
     - Run as non-root: `USER cynic`
     - Add health check: `HEALTHCHECK --interval=30s --timeout=5s CMD /app/healthcheck.sh`
     - Scan for vulnerabilities: `docker scan cynic-kernel:latest`
  2. Enhance docker-compose.yml:
     - Add resource limits: CPU, memory, disk I/O
     - Add restart policy: `restart_policy: { condition: on-failure, max_retries: 3 }`
     - Add logging: `logging: { driver: json-file, options: { max-size: '10m' } }`
     - Add health check: curl /health endpoint
     - Add secrets management: use .env.vault instead of .env
  3. Test: build image locally, verify no vulnerabilities, verify health check works
  4. Document: `.github/DEPLOYMENT.md` — how to deploy to production
  5. Target: Image passes security scan, health checks pass, resources bounded

**Status**: Basic setup done, hardening not done.

---

### HIGH (Should fix before Phase 2 production)

#### DEPLOY-H1-001: No Observability (Metrics, Logging, Tracing)

- **Title**: Production system invisible; can't debug issues or monitor health
- **Severity**: HIGH
- **Impact**: Outages undetected. Performance degradation invisible. Root cause analysis impossible. SRE team has no tools.
- **Effort**: 12 hours (core setup)
- **Root Cause**: Focus on functionality over operations. No metrics instrumentation. No centralized logging.
- **Evidence**:
  - API endpoints exist but no Prometheus endpoint
  - Logs written to stdout, no aggregation
  - No distributed tracing
  - Task 6 (MEDIUM) mentioned observability but not high priority
- **Action**:
  1. Add Prometheus instrumentation:
     - Install `prometheus-client` library
     - Add metrics to key code paths:
       - `chat_requests_total` (counter) — ChatSession.add_message()
       - `chat_response_latency_ms` (histogram) — time to generate response
       - `event_bus_events_published` (counter) — per bus
       - `q_learning_updates` (counter) — per dog
     - Expose `/metrics` endpoint (FastAPI integration)
  2. Add centralized logging:
     - Install `python-json-logger`
     - Replace all `print()` and `logging.debug()` with structured JSON logs
     - Add context fields: request_id, session_id, user_id, timestamp
     - Forward logs to ELK (Elasticsearch, Logstash, Kibana) or Datadog
  3. Add distributed tracing:
     - Install `opentelemetry-instrumentation-fastapi`
     - Instrument API requests: trace each request end-to-end
     - Export to Jaeger or Tempo
  4. Add dashboard:
     - Grafana dashboard showing: CPU, memory, API latency, event bus throughput
     - Alert on: p99 latency > 500ms, error rate > 1%, memory > 80%
  5. Target: Operator can diagnose any production issue within 5 minutes

**Status**: Not started. Critical for operational readiness.

---

#### DEPLOY-H1-002: No Automated Database Backups

- **Title**: PostgreSQL data unprotected; data loss risk
- **Severity**: HIGH
- **Impact**: Container restart loses all data. No recovery procedure. Unacceptable for production.
- **Effort**: 4 hours
- **Root Cause**: Docker Compose spins up PostgreSQL but no backup strategy. Ephemeral by default.
- **Evidence**:
  - docker-compose.yml doesn't persist Postgres data volume
  - No backup script
  - No restore testing
- **Action**:
  1. Add persistent volume to docker-compose:
     ```yaml
     volumes:
       postgres_data:
         driver: local
     services:
       postgres:
         volumes:
           - postgres_data:/var/lib/postgresql/data
     ```
  2. Create backup script (`scripts/backup_db.sh`):
     - Daily `pg_dump` to compressed file
     - Upload to S3 (AWS) or GCS (Google Cloud)
     - Retention: keep last 30 days
  3. Create restore script (`scripts/restore_db.sh`):
     - Given backup file, restore to running database
     - Verify restore succeeded (table counts, integrity check)
  4. Test restore: monthly restore-and-verify drill
  5. Document in `docs/RUNBOOK.md`
  6. Target: RPO 24h, RTO < 1h

**Status**: Not started.

---

#### DEPLOY-H1-003: No Container Registry / Image Tagging

- **Title**: No versioned Docker images; can't rollback deployments
- **Severity**: HIGH
- **Impact**: Release management broken. Can't track which image is deployed. Can't rollback to previous version.
- **Effort**: 4 hours
- **Root Cause**: Docker image built locally but not pushed to registry. No version tagging strategy.
- **Evidence**:
  - Dockerfile exists but no build/push in CI
  - No image registry (DockerHub, ECR, GHCR)
  - No versioning strategy
- **Action**:
  1. Set up image registry:
     - Option A: Docker Hub (free, public image)
     - Option B: GitHub Container Registry (GHCR, private, GitHub-integrated)
     - Option C: AWS ECR (paid, enterprise-grade)
     - Recommend: GHCR for simplicity
  2. Update CI pipeline:
     - Build image: `docker build -t ghcr.io/cynic/kernel:latest -t ghcr.io/cynic/kernel:v0.1.0 .`
     - Push on merge to main: `docker push ghcr.io/cynic/kernel:v0.1.0`
     - Tag with commit SHA and version
  3. Versioning strategy: semantic versioning (v0.1.0, v0.2.0, etc.)
  4. Deployment: always reference specific version (never `latest`)
  5. Target: Reproducible deployments, easy rollbacks

**Status**: Not started.

---

#### DEPLOY-H1-004: Load Testing Not Done; Unknown Scaling Limits

- **Title**: No performance baseline; can't validate scaling assumptions
- **Severity**: HIGH
- **Impact**: Deploy to production and discover it can only handle 10 RPS (unknown until failure). No SLA targets.
- **Effort**: 6 hours
- **Root Cause**: API works in dev but never stress-tested. Task 6 (MEDIUM) mentioned profiling but deferred.
- **Evidence**:
  - API endpoints functional
  - No load test tools (k6, wrk) set up
  - No latency targets defined
- **Action**:
  1. Create k6 load test (`tests/load/chat_api.js`):
     - Scenario 1: Ramp up to 50 RPS over 1 minute, sustain 5 minutes
     - Scenario 2: Burst to 200 RPS for 10 seconds (spike test)
     - Measure: p50, p95, p99 latencies, error rate, throughput
  2. Run locally: establish baseline on development machine
     - Target p99: 500ms, error rate: 0.1%
  3. Run in Docker: verify no degradation vs local
  4. Run in cloud: scale PostgreSQL horizontally, measure throughput ceiling
  5. Document baseline in `docs/PERFORMANCE.md`
  6. Set up continuous benchmarking: run monthly, track regression
  7. Target: Documented SLA (e.g., "handles 100 concurrent users, p99 < 500ms")

**Status**: Not started.

---

#### DEPLOY-H1-005: No Deployment Runbook / Procedures

- **Title**: Operators don't know how to deploy, scale, or recover
- **Severity**: HIGH
- **Impact**: Manual deployments error-prone. No documented procedures. Knowledge isolated in one engineer's head.
- **Effort**: 4 hours
- **Root Cause**: Focus on building, not operating. No operations documentation.
- **Evidence**:
  - Docker Compose set up but no deployment guide
  - No scaling procedures documented
  - No incident response playbook
- **Action**:
  1. Create `docs/RUNBOOK.md`:
     - **Deploy**: step-by-step to push new version
     - **Rollback**: revert to previous version
     - **Scale**: add more instances, load balancer setup
     - **Monitor**: check logs, metrics, health
     - **Incident Response**: common issues and solutions
  2. Create `docs/TROUBLESHOOTING.md`:
     - High CPU → likely event bus contention, check metrics
     - High latency → check database, event handler logs
     - Memory leak → check chat session cleanup
  3. Create `docs/MAINTENANCE.md`:
     - Daily: check logs, verify no errors
     - Weekly: run load test, verify baseline
     - Monthly: backup restore drill, security scan
  4. Test documentation: new operator should be able to deploy without help
  5. Target: Junior engineer can deploy to production using runbook only

**Status**: Not started.

---

### MEDIUM (Nice to have)

#### DEPLOY-M1-001: Kubernetes Manifest Not Ready

- **Title**: No K8s deployment manifests; can't scale horizontally
- **Severity**: MEDIUM
- **Impact**: Locked into single-machine Docker Compose. Can't handle traffic spikes. Can't do rolling updates.
- **Effort**: 12 hours
- **Root Cause**: Focused on local Docker development. Kubernetes seems "too much" for Phase 1.
- **Evidence**:
  - docker-compose.yml works but doesn't scale
  - No K8s manifests (deployment, service, ingress, configmap)
- **Action**:
  1. Create K8s manifests (`k8s/` directory):
     - `deployment.yaml`: cynic-kernel pod spec, replicas, resource requests/limits
     - `service.yaml`: expose API on port 8000, type LoadBalancer
     - `configmap.yaml`: config values (not secrets)
     - `secret.yaml`: environment variables (encrypted)
     - `ingress.yaml`: route requests, TLS termination
  2. Create `k8s/kustomization.yaml`: base + overlays (dev, staging, prod)
  3. Test locally: `minikube start && kubectl apply -k k8s/`
  4. Document: `docs/KUBERNETES.md` — how to deploy to K8s
  5. Target: `kubectl apply -f k8s/` deploys full system

**Status**: Not started. Medium priority for scalability.

---

#### DEPLOY-M1-002: Security Scanning Not Automated

- **Title**: No vulnerability scanning; unknown security risks
- **Severity**: MEDIUM
- **Impact**: Dependencies with known vulnerabilities used unknowingly. Container images not scanned. Secret scanning not active.
- **Effort**: 4 hours
- **Root Cause**: Security hardening deferred. No scanning tools configured.
- **Evidence**:
  - No SBOM (software bill of materials)
  - No Dependabot alerts
  - No container scanning
- **Action**:
  1. Enable Dependabot:
     - GitHub Settings → Enable Dependabot
     - Auto-creates PRs for dependency updates
     - Notifies of vulnerabilities
  2. Add container scanning to CI:
     - `docker scan cynic-kernel:latest` after build
     - Fail if critical vulnerabilities found
  3. Add secret scanning:
     - GitHub: Settings → Secret scanning (auto-detects secrets in commits)
     - Pre-commit hook: scan for hardcoded credentials
  4. Document security baseline in `docs/SECURITY.md`
  5. Target: Zero known vulnerabilities, automated detection

**Status**: Not started.

---

#### DEPLOY-M1-003: No Feature Flags / Canary Deployment

- **Title**: Deployments are all-or-nothing; risky for production changes
- **Severity**: MEDIUM
- **Impact**: New features deployed to all users at once. Bugs affect everyone. No gradual rollout capability.
- **Effort**: 6 hours
- **Root Cause**: Deployment strategy hasn't been designed. No feature flag infrastructure.
- **Evidence**:
  - docker-compose treats all containers same
  - No canary / blue-green deployment setup
- **Action**:
  1. Set up feature flag service:
     - Option A: LaunchDarkly (SaaS, expensive)
     - Option B: Unleash (open-source, self-hosted)
     - Recommend: Unleash for Phase 2
  2. Integrate into API:
     - Check feature flag before executing new code path
     - Example: `if feature_flag("enable_learning_loops"): start_sona()`
  3. Deployment strategy:
     - Deploy with new feature hidden (flag=false)
     - Monitor for 1 hour
     - Gradually enable: 10% → 50% → 100%
     - If issues: disable instantly
  4. Document canary strategy in `docs/DEPLOYMENT.md`
  5. Target: Zero-downtime deployments, rapid rollback

**Status**: Not started.

---

#### DEPLOY-M1-004: Cost Optimization Not Done

- **Title**: Unknown cloud cost; likely overspending
- **Severity**: MEDIUM (low operational impact, financial impact)
- **Impact**: Wasted cloud budget. No visibility into per-component costs.
- **Effort**: 4 hours
- **Root Cause**: Running in docker-compose on single machine. Costs not monitored.
- **Evidence**:
  - No cost dashboard
  - No resource limits enforcement
  - Unknown whether infrastructure is right-sized
- **Action**:
  1. If using cloud (AWS, GCP, Azure):
     - Set up cost monitoring: CloudWatch / Cost Explorer
     - Tag resources: env=dev, component=database, owner=cynic
     - Set budget alerts: notify if costs exceed $500/month
  2. Right-size resources:
     - CPU/memory may be oversized
     - Use profiler to measure actual usage
     - Reduce resource requests if unused
  3. Document in `docs/OPERATIONS.md`
  4. Target: Costs tracked, budgets enforced, resources optimized

**Status**: Not started.

---

### LOW (Cosmetic / Future)

#### DEPLOY-L1-001: No API Documentation / OpenAPI Spec

- **Title**: API endpoints not documented; hard for external consumers
- **Severity**: LOW
- **Impact**: Third-party integrations difficult. API contract not clear.
- **Effort**: 4 hours
- **Root Cause**: Focus on delivery. OpenAPI/Swagger not set up.
- **Evidence**:
  - FastAPI app exists but no OpenAPI spec generated
  - No /docs endpoint exposed
  - No API schema versioning
- **Action**:
  1. FastAPI generates OpenAPI automatically if enabled
  2. Expose `/docs` (Swagger UI) and `/redoc` (ReDoc) endpoints
  3. Document API responses with OpenAPI schema annotations
  4. Generate client SDK from spec (JavaScript, Python)
  5. Target: Consumers can learn API from interactive docs

**Status**: FastAPI capable, docs not enabled.

---

#### DEPLOY-L1-002: No Disaster Recovery Plan

- **Title**: No documented recovery procedures; disaster response ad-hoc
- **Severity**: LOW
- **Impact**: If major outage, response chaotic. No RTO/RPO targets.
- **Effort**: 6 hours
- **Root Cause**: Disaster recovery feels premature for Phase 1. But should document for Phase 2.
- **Evidence**:
  - No DR procedures documented
  - No tested recovery paths
- **Action**:
  1. Create `docs/DISASTER_RECOVERY.md`:
     - RTO target: restore service within 1 hour
     - RPO target: lose at most 24 hours of data
     - Scenarios: database corruption, API crash, infrastructure failure
     - Recovery procedure for each
  2. Test quarterly: simulate failure, execute recovery, verify success
  3. Target: Operators confident in recovery capability

**Status**: Not started.

---

## Deployment Debt Reduction Strategy

### Recommended Fix Order (by Risk)

**Phase 2 Blocking Fixes (First Week)**:
1. DEPLOY-T1-001: CI/CD pipeline (6h) — Quality gates on merges
2. DEPLOY-T1-002: Pinned dependencies (2h) — Reproducible environments
3. DEPLOY-T1-003: Docker hardening (8h) — Security baseline

**Subtotal**: 16 hours

**Phase 2 Production-Readiness Fixes (Week 2-3)**:
4. DEPLOY-H1-001: Observability (12h) — Operational visibility
5. DEPLOY-H1-002: DB backups (4h) — Data safety
6. DEPLOY-H1-003: Container registry (4h) — Versioning and rollbacks
7. DEPLOY-H1-005: Runbook (4h) — Operational procedures

**Subtotal**: 24 hours

**Phase 2 Scaling Fixes (Week 4)**:
8. DEPLOY-H1-004: Load testing (6h) — Performance baseline
9. DEPLOY-M1-001: Kubernetes (12h) — Horizontal scaling

**Subtotal**: 18 hours

**Total Effort to Clear BLOCKER + HIGH**: 40 hours (~1.5 weeks, 1 DevOps engineer)

---

## Pre-Production Checklist

Before deploying to production, verify:

- [ ] CI/CD pipeline: all tests pass, no lint errors
- [ ] Dependencies: locked, no known vulnerabilities
- [ ] Docker: multi-stage, non-root user, health checks
- [ ] Database: persistent volume, automated backups, restore tested
- [ ] Observability: metrics endpoint working, logs aggregated, alerting configured
- [ ] Load test: p99 latency < 500ms at 100 RPS
- [ ] Runbook: new operator can deploy without support
- [ ] Security: container scanned, no secrets in code, secrets encrypted
- [ ] Documentation: SETUP.md, ARCHITECTURE.md, DEPLOYMENT.md all complete

**Confidence**: 58% (φ-bounded)

*sniff* Infrastructure debt is systematic. Phase 2 must address the BLOCKER items before production exposure.
