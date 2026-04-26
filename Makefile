# CYNIC — Universal Development Pipeline
# Works for Claude Code, Gemini CLI, humans, any machine.
# One entry point per stage. No discipline required — the structure enforces the flow.
#
# Usage:
#   make check    — build + test + clippy (validate before commit)
#   make commit   — check + commit (validated commit)
#   make ship     — commit + push (pre-push re-validates)
#   make deploy   — ship + backup DB + restart kernel + verify
#   make deploy-only — deploy existing binary WITHOUT commit/push/check (fast-path)
#   make hotfix   — deploy WITHOUT push (emergency only — skips ship)
#   make rollback — restore previous binary
#   make restore  — restore DB from backup file (F=path)
#   make e2e      — end-to-end test against running kernel
#   make status   — full system dashboard
#   make runtime-truth — PID/PPID/binary/version/state/port truth for critical processes
#   make runtime-check — fail on runtime drift or duplicate critical processes; warn on minimal unit profiles
#   make install-systemd-units — symlink repo-managed user units into ~/.config/systemd/user
#   make verify-systemd-units — fail if installed user units drift from the repo
#   make backup   — manual DB backup
#   make test-gates — R21: verify lint gates catch known violations
#   make clean    — cargo clean (fix SIGSEGV/LLVM crashes)

SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
MAKEFLAGS += --warn-undefined-variables

# ── Environment ──────────────────────────────────────────────
PROJECT_DIR := $(shell git rev-parse --show-toplevel 2>/dev/null || pwd)

# Source env vars (CYNIC_REST_ADDR, CYNIC_API_KEY, etc.)
define source_env
	source ~/.cargo/env 2>/dev/null || true
	source ~/.cynic-env 2>/dev/null || true
	export RUST_MIN_STACK=67108864  # 64 MiB: rmcp serde monomorphization (A1 debt). Source of truth: .cargo/config.toml
endef

# ── Stage 1: Validate ───────────────────────────────────────
.PHONY: check
check:
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  CYNIC check — build + test + clippy"
	@echo "══════════════════════════════════════════"
	@$(MAKE) --no-print-directory verify-hooks
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test -p cynic-kernel --lib --release  # A1 debt: debug test binary hits lld invalid-symbol-index bug; --release avoids it
	@$(MAKE) --no-print-directory lint-rules
	@$(MAKE) --no-print-directory lint-drift
	@$(MAKE) --no-print-directory lint-subprocess-env
	@$(MAKE) --no-print-directory lint-security
	@echo ""; echo "▶ Security audit (cargo audit)..."
	cargo audit --deny warnings
	@if surreal is-ready --endpoint http://localhost:8000 2>/dev/null; then \
		echo ""; echo "▶ Integration tests (SurrealDB available)..."; \
		cargo test -p cynic-kernel --test integration_storage && \
		cargo test -p cynic-kernel --test storage_contract; \
	else \
		echo ""; echo "⚠ SurrealDB not running — skipping integration tests"; \
	fi

.PHONY: lint-rules
lint-rules: ## Grep-enforceable CLAUDE.md rules — uses grep (not rg alias, which is unavailable in Make subshells)
# Exemptions: probe/ (boot-time hardware discovery), infra/ (health loop, config validation)
# These modules DO raw HTTP but are not backend adapters — they serve infrastructure purposes.
# Requires .ONESHELL for set +e scope to work across the whole recipe.
	@echo ""
	@echo "▶ Checking grep-enforceable rules..."
	@set +e; FAIL=0; \
	K1=$$(grep -rn '#\[cfg(' cynic-kernel/src/domain/ --include='*.rs' | grep -v '//' | grep -v '#\[cfg(test)\]' | grep -v '#\[cfg_attr(test'); \
	if [ -n "$$K1" ]; then echo "FAIL K1: #[cfg] in domain/ (domain purity):"; echo "$$K1"; FAIL=1; fi; \
	R1=$$(grep -rn '/home/\|/Users/' cynic-kernel/src/ scripts/ .claude/hooks/ --include='*.rs' --include='*.sh' --include='*.toml' | grep -v '//' | grep -v 'target/'); \
	if [ -n "$$R1" ]; then echo "FAIL R1: hardcoded absolute paths:"; echo "$$R1"; FAIL=1; fi; \
	R2=$$(grep -rn '\.ok()' cynic-kernel/src/ --include='*.rs' | grep -v '//' | grep -v '/target/' | grep -v '#\[cfg(test)\]' | grep -v 'mod tests' \
		| grep -v 'parse()\.ok()' | grep -v 'env::var.*\.ok()' | grep -v 'inspect_err' \
		| grep -v 'to_str()\.ok()' | grep -v 'from_utf8.*\.ok()' | grep -v 'filter_map.*\.ok()' \
		| grep -v 'create_dir_all.*\.ok()' | grep -v 'remove_file.*\.ok()' \
		| grep -v '\.lock()\.ok()' | grep -v '\.read()\.ok()' \
		| grep -v '\.output()' | grep -v 'read_to_string.*\.ok()'); \
	if [ -n "$$R2" ]; then \
		while IFS= read -r okline; do \
			FILE=$$(echo "$$okline" | cut -d: -f1); LINENUM=$$(echo "$$okline" | cut -d: -f2); \
			NEARBY=$$(sed -n "$$((LINENUM>8?LINENUM-8:1)),$$(( LINENUM+3 ))p" "$$FILE" 2>/dev/null || true); \
			if echo "$$NEARBY" | grep -q 'inspect_err\|tracing::\|eprintln!\|warn!\|env::var\|WHY:\|K14\|poison'; then continue; fi; \
			if echo "$$FILE" | grep -q 'probe/\|probes/'; then continue; fi; \
			echo "FAIL R2: .ok() without adjacent logging: $$okline"; FAIL=1; \
		done <<< "$$R2"; \
	fi; \
	K2=$$(grep -rn 'reqwest' cynic-kernel/src/domain/ cynic-kernel/src/api/ cynic-kernel/src/main.rs --include='*.rs' | grep -v '//' | grep -v 'api/mcp/proxy.rs'); \
	if [ -n "$$K2" ]; then echo "FAIL K2: reqwest outside backends/storage:"; echo "$$K2"; FAIL=1; fi; \
	PIPELINE_FNS="format_crystal_context compute_qscore trimmed_mean"; \
	for FN in $$PIPELINE_FNS; do \
		HITS=$$(grep -rn "fn $$FN" cynic-kernel/src/api/ --include='*.rs' | grep -v '//'); \
		if [ -n "$$HITS" ]; then echo "FAIL K3/K11: pipeline function '$$FN' reimplemented in api/:"; echo "$$HITS"; FAIL=1; fi; \
	done; \
	K4=$$(grep -rn 'trait \w*Port' cynic-kernel/src/domain/ --include='*.rs' | sed 's/.*trait //' | sed 's/<.*//' | sed 's/ .*//' | sort | uniq -d); \
	if [ -n "$$K4" ]; then echo "FAIL K4: duplicate trait names in domain/:"; echo "$$K4"; FAIL=1; fi; \
	K5=$$(grep -rn 'serde_json::Value\|reqwest::\|surrealdb::\|axum::' cynic-kernel/src/domain/ --include='*.rs' | grep -v '//' | grep -v '#\[cfg(test)\]'); \
	if [ -n "$$K5" ]; then echo "FAIL K5: infra type leakage in domain/:"; echo "$$K5"; FAIL=1; fi; \
	K12=$$(grep -rn '#\[allow(' cynic-kernel/src/ --include='*.rs' | grep -v '/target/' | grep -v '#\[cfg(test)\]' | grep -v '#\[cfg_attr(test'); \
	if [ -n "$$K12" ]; then \
		while IFS= read -r allowline; do \
			FILE=$$(echo "$$allowline" | cut -d: -f1); LINENUM=$$(echo "$$allowline" | cut -d: -f2); \
			NEARBY=$$(sed -n "$$(( LINENUM>2 ? LINENUM-2 : 1)),$$(( LINENUM+2 ))p" "$$FILE" 2>/dev/null); \
			if ! echo "$$NEARBY" | grep -q 'WHY:'; then \
				echo "FAIL K12: #[allow] without WHY: comment: $$allowline"; FAIL=1; \
			fi; \
		done <<< "$$K12"; \
	fi; \
	ADDED=$$(git diff --staged 2>/dev/null | grep '^+' | grep -v '^+++' | grep -v 'RAW_SECRETS=' | grep -v 'HARDCODED_CREDS=' | grep -v 'REAL_INFRA=' | grep -v 'grep -iE .*api\[_-\]?key' | grep -v 'grep -iE .*AIza' | grep -v 'grep -E .*100\\\.'); \
	RAW_SECRETS=$$(printf '%s\n' "$$ADDED" | grep -iE 'AIza[[:alnum:]_-]{10,}|hf_[[:alnum:]_-]{10,}|sk-[[:alnum:]_-]{10,}'); \
	HARDCODED_CREDS=$$(printf '%s\n' "$$ADDED" | grep -iE '(api[_-]?key|token|password|secret)[^[:alnum:]]*[:=][[:space:]]*["'"'"']?[^$$<[:space:]][^"'"'"'[:space:]]{6,}' | grep -viE 'change-me|your-api-key-here|example|placeholder|dummy|fake|test-key|redacted|CancellationToken'); \
	if [ -n "$$RAW_SECRETS$$HARDCODED_CREDS" ]; then echo "FAIL Security: possible hardcoded secrets in staged changes:"; echo "$$RAW_SECRETS"; echo "$$HARDCODED_CREDS"; FAIL=1; fi; \
	REAL_INFRA=$$(printf '%s\n' "$$ADDED" | grep -E '100\.(74|75|119)\.[0-9]{1,3}\.[0-9]{1,3}'); \
	if [ -n "$$REAL_INFRA" ]; then echo "FAIL Security: real Tailscale IPs in staged changes:"; echo "$$REAL_INFRA"; FAIL=1; fi; \
	if [ $$FAIL -eq 0 ]; then echo "✓ All grep-enforceable rules pass"; fi; \
	exit $$FAIL

.PHONY: lint-services
lint-services: ## Compare running user services vs expected list — catches zombie/missing services
	@echo ""
	@echo "▶ Checking service state..."
	@EXPECTED="docs/ops/expected-services.txt"; \
	if [ ! -f "$$EXPECTED" ]; then echo "⚠ $$EXPECTED not found — skipping"; exit 0; fi; \
	FAIL=0; \
	while IFS= read -r svc; do \
		case "$$svc" in \#*|"") continue;; esac; \
		if ! systemctl --user is-active --quiet "$$svc" 2>/dev/null; then \
			echo "✗ MISSING: $$svc (expected active, not found)"; FAIL=1; \
		fi; \
	done < "$$EXPECTED"; \
	for svc in $$(systemctl --user list-units --type=service --state=active --no-legend 2>/dev/null | awk '{print $$1}' | grep -E "cynic|llama|surreal|kairos|cloudflare"); do \
		if ! grep -q "^$$svc$$" "$$EXPECTED" 2>/dev/null; then \
			echo "✗ UNEXPECTED: $$svc (running but not in expected list)"; FAIL=1; \
		fi; \
	done; \
	if [ $$FAIL -eq 0 ]; then echo "✓ All services match expected state"; fi; \
	exit $$FAIL

.PHONY: lint-drift
lint-drift: ## Detect config/code/docs drift — names vs reality, dead modules, phantom skills, unwired hooks
	@echo ""
	@echo "▶ Checking for drift..."
	@set +e; FAIL=0; \
	BACKENDS="$${HOME}/.config/cynic/backends.toml"; \
	if [ -f "$$BACKENDS" ]; then \
		while IFS= read -r line; do \
			NAME=$$(echo "$$line" | grep -oP '(?<=\[backend\.)[^]]+' | sed 's/\.remediation//'); \
			[ -z "$$NAME" ] && continue; \
			MODEL=$$(sed -n "/\[backend\.$${NAME}\]/,/^\[/p" "$$BACKENDS" | grep '^model' | head -1 | sed 's/.*= *"//;s/".*//'); \
			[ -z "$$MODEL" ] && continue; \
			if [ "$$MODEL" = "auto" ]; then continue; fi; \
			PREFIX=$$(echo "$$NAME" | sed 's/[-_].*//' | tr -d '.' | tr '[:upper:]' '[:lower:]'); \
			MODEL_NODOTS=$$(echo "$$MODEL" | tr -d '.' | tr '[:upper:]' '[:lower:]'); \
			if ! echo "$$MODEL_NODOTS" | grep -q "$$PREFIX"; then \
				echo "WARN Drift: Dog '$$NAME' → model '$$MODEL' (name prefix '$$PREFIX' not in model)"; \
				FAIL=1; \
			fi; \
		done < <(grep '^\[backend\.' "$$BACKENDS" | grep -v '\.remediation\]'); \
	else \
		echo "SKIP: backends.toml not found at $$BACKENDS"; \
	fi; \
	DORMANT=$$(grep -nE '^\s*//\s*pub mod' $(PROJECT_DIR)/cynic-kernel/src/domain/mod.rs | grep -v 'DORMANT:'); \
	if [ -n "$$DORMANT" ]; then echo "FAIL Drift: commented module without DORMANT tag:"; echo "$$DORMANT"; FAIL=1; fi; \
	WORKFLOW_SKILLS=$$(sed -n '/^## Workflow Triggers/,/^## /p' $(PROJECT_DIR)/.claude/rules/workflow.md | grep -oP '(?<=`/)[a-z][-a-z0-9:]*(?=`)' | sort -u); \
	for SKILL in $$WORKFLOW_SKILLS; do \
		FOUND=0; \
		if echo "$$SKILL" | grep -q ':'; then \
			NS=$$(echo "$$SKILL" | cut -d: -f1); NAME=$$(echo "$$SKILL" | cut -d: -f2); \
			[ -d "$${HOME}/.claude/commands/$${NS}/$${NAME}" ] && FOUND=1; \
			[ -f "$${HOME}/.claude/commands/$${NS}/$${NAME}.md" ] && FOUND=1; \
		else \
			[ -f "$(PROJECT_DIR)/.claude/commands/$${SKILL}.md" ] && FOUND=1; \
		fi; \
		if [ $$FOUND -eq 0 ]; then \
			echo "FAIL Drift: skill '/$${SKILL}' in workflow.md but not found on disk"; FAIL=1; \
		fi; \
	done; \
	for HOOK in $(PROJECT_DIR)/.claude/hooks/*.sh; do \
		HOOK_BASE=$$(basename "$$HOOK"); \
		if ! grep -q "$$HOOK_BASE" $(PROJECT_DIR)/.claude/settings.json 2>/dev/null && \
		   ! grep -q "$$HOOK_BASE" $(PROJECT_DIR)/.claude/settings.local.json 2>/dev/null; then \
			echo "WARN Drift: hook '$$HOOK_BASE' on disk but not wired in settings.json or settings.local.json"; FAIL=1; \
		fi; \
	done; \
	STORES=$$(grep -oP 'async fn \Kstore_\w+' $(PROJECT_DIR)/cynic-kernel/src/domain/storage/mod.rs | sort -u); \
	for STORE in $$STORES; do \
		ENTITY=$$(echo "$$STORE" | sed 's/^store_//'); \
		STEM=$$(echo "$$ENTITY" | sed 's/y$$//' | sed 's/ing$$//'); \
		CORE=$$(echo "$$ENTITY" | sed 's/^.*_//'); \
		if ! grep -qE "async fn (list_|get_|load_|query_|count_|search_|find_)\w*($$STEM|$$CORE)" $(PROJECT_DIR)/cynic-kernel/src/domain/storage/mod.rs; then \
			echo "FAIL Rule 33: '$$STORE' has no read path (searched '$$STEM' or '$$CORE')"; FAIL=1; \
		fi; \
	done; \
	ROUTES=$$(grep -oP '\.route\("\K[^"]+' $(PROJECT_DIR)/cynic-kernel/src/api/rest/mod.rs); \
	for ROUTE in $$ROUTES; do \
		if ! grep -qF "$$ROUTE" $(PROJECT_DIR)/API.md 2>/dev/null; then \
			echo "FAIL Drift: route '$$ROUTE' registered in code but missing from API.md"; FAIL=1; \
		fi; \
	done; \
	REF_DOGS=$$(sed -n '/^## Dogs/,/^##/p' $(PROJECT_DIR)/.claude/rules/reference.md | grep -oP '(?<=\| )[a-z][-a-z0-9]+(?= +\|)' | sort -u); \
	LIVE_DOGS="deterministic-dog"; \
	if [ -f "$$BACKENDS" ]; then \
		LIVE_DOGS=$$(printf '%s\n' "$$LIVE_DOGS" $$(grep -oP '(?<=\[backend\.)[^]]+' "$$BACKENDS" | grep -v '\.remediation') | sort -u); \
	fi; \
	REF_ONLY=$$(comm -23 <(echo "$$REF_DOGS") <(echo "$$LIVE_DOGS")); \
	LIVE_ONLY=$$(comm -13 <(echo "$$REF_DOGS") <(echo "$$LIVE_DOGS")); \
	if [ -n "$$REF_ONLY" ]; then echo "FAIL D4: Dogs in reference.md but not active: $$REF_ONLY"; FAIL=1; fi; \
	if [ -n "$$LIVE_ONLY" ]; then echo "FAIL D4: Dogs active but not in reference.md: $$LIVE_ONLY"; FAIL=1; fi; \
	KERN="$(PROJECT_DIR)/cynic-kernel/src"; \
	DISSOLVED=$$(grep -rn "dissolved\|Dissolved" "$$KERN/storage/" "$$KERN/infra/" --include='*.rs' 2>/dev/null | grep -v 'test\|//\|Display\|FromStr\|Serialize\|Deserialize' | grep -i 'dissolved' | wc -l); \
	if [ "$$DISSOLVED" -eq 0 ]; then echo "FAIL K15: CrystalState::Dissolved defined but never produced"; FAIL=1; fi; \
	COMPLIANCE_GATES=$$(grep -rn 'compliance.*deny\|compliance.*block\|compliance.*throttle\|compliance.*reject' "$$KERN/" --include='*.rs' 2>/dev/null | grep -v 'test\|//' | wc -l); \
	if [ "$$COMPLIANCE_GATES" -eq 0 ]; then echo "WARN K15: compliance_score computed but no gate enforces threshold"; fi; \
	CV_READERS=$$(grep -rn 'contributing_verdicts' "$$KERN/" --include='*.rs' 2>/dev/null | grep -v 'test\|//\|push\|insert\|store\|array::union\|Vec<String>\|struct\|pub ' | wc -l); \
	if [ "$$CV_READERS" -eq 0 ]; then echo "WARN K15: contributing_verdicts stored but never read"; fi; \
	EVENT_VARIANTS=$$(grep -oP 'KernelEvent::\K\w+' "$$KERN/domain/events.rs" 2>/dev/null | sort -u); \
	EVENT_HANDLERS=$$(grep -oP 'KernelEvent::\K\w+' "$$KERN/infra/tasks/runtime_loops.rs" 2>/dev/null | sort -u); \
	for EV in $$EVENT_VARIANTS; do \
		if ! echo "$$EVENT_HANDLERS" | grep -q "$$EV"; then \
			EMITTERS=$$(grep -rn "KernelEvent::$$EV" "$$KERN/" --include='*.rs' 2>/dev/null | grep -v 'test\|//\|match\|=>' | wc -l); \
			if [ "$$EMITTERS" -gt 0 ]; then \
				echo "WARN K15: KernelEvent::$$EV emitted ($$EMITTERS site(s)) but not matched in event_consumer"; \
			fi; \
		fi; \
	done; \
	for RULE in $(PROJECT_DIR)/.claude/rules/*.md; do \
		grep -oP '`[a-zA-Z_/]+\.(rs|toml|sh|md)`' "$$RULE" 2>/dev/null | tr -d '`' | while read -r REF; do \
			if echo "$$REF" | grep -qE '\.rs$$' && ! find "$$KERN" -path "*$$REF" 2>/dev/null | grep -q .; then \
				if ! find "$(PROJECT_DIR)" -path "*$$REF" 2>/dev/null | grep -q .; then \
					echo "WARN Drift: '$$REF' referenced in $$(basename $$RULE) but not found on disk"; \
				fi; \
			fi; \
		done; \
	done; \
	PORT_METHODS=$$(grep -P '^\s+async fn \w+' $(PROJECT_DIR)/cynic-kernel/src/domain/storage/mod.rs | grep -oP 'fn \K\w+' | sort); \
	RECON_METHODS=$$(grep -P '^\s+async fn \w+' $(PROJECT_DIR)/cynic-kernel/src/storage/reconnectable.rs | grep -oP 'fn \K\w+' | sort); \
	for PM in $$PORT_METHODS; do \
		if ! echo "$$RECON_METHODS" | grep -qw "$$PM"; then \
			echo "FAIL K17: StoragePort::$$PM not forwarded in ReconnectableStorage"; FAIL=1; \
		fi; \
	done; \
	if [ $$FAIL -eq 0 ]; then echo "✓ No drift detected"; fi; \
	exit $$FAIL

.PHONY: lint-subprocess-env
lint-subprocess-env: ## R23 gate: hooks/scripts/Rust subprocess env is explicit (RUST_MIN_STACK, --edition)
	@bash $(PROJECT_DIR)/scripts/lint-subprocess-env.sh

.PHONY: lint-security
lint-security: ## G1 gate: 0 OPEN findings in CRITICAL or HIGH sections of findings tracker
	@echo ""
	@echo "▶ Checking security findings (G1 gate)..."
	@TRACKER="$(PROJECT_DIR)/docs/audit/CYNIC-FINDINGS-TRACKER.md"; \
	if [ ! -f "$$TRACKER" ]; then echo "SKIP: findings tracker not found"; exit 0; fi; \
	OPEN=$$(awk '/^## CRITICAL/{s=1} /^## HIGH/{s=1} /^## MEDIUM/{s=0} s && /^\|/ && !/^\| #/ && !/^\|---/ && / OPEN/' "$$TRACKER"); \
	if [ -n "$$OPEN" ]; then \
		echo "FAIL G1: CRIT/HIGH findings still OPEN:"; echo "$$OPEN"; exit 1; \
	else \
		echo "✓ G1: 0 OPEN findings in CRITICAL/HIGH"; \
	fi

.PHONY: test-gates
test-gates: ## R21: Verify lint gates catch known violations (inject → check → restore)
	@echo ""
	@echo "══════════════════════════════════════════"
	@echo "  R21: Gate falsification tests"
	@echo "══════════════════════════════════════════"
	@set +e; PASS=0; FAIL=0; \
	\
	echo ""; echo "── lint-rules ──"; \
	\
	echo "[K1] Injecting #[cfg(feature)] in domain/..."; \
	TARGET="$(PROJECT_DIR)/cynic-kernel/src/domain/mod.rs"; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo '#[cfg(feature = "gate_test_k1")] mod _phantom;' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K1 gate MISSED #[cfg] in domain/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K1 gate caught #[cfg] in domain/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[K2] Injecting reqwest in domain/..."; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo 'use reqwest;' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K2 gate MISSED reqwest in domain/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K2 gate caught reqwest in domain/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[K5] Injecting serde_json::Value in domain/..."; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo 'fn _k5_test() -> serde_json::Value { todo!() }' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K5 gate MISSED serde_json::Value in domain/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K5 gate caught serde_json::Value in domain/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[K5+] Injecting axum:: in domain/..."; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo 'fn _k5_test() -> axum::Router { todo!() }' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K5 gate MISSED axum:: in domain/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K5 gate caught axum:: in domain/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[K12] Injecting #[allow] without WHY:..."; \
	ALLOW_TARGET="$(PROJECT_DIR)/cynic-kernel/src/main.rs"; \
	cp "$$ALLOW_TARGET" "$$ALLOW_TARGET.gate-bak"; \
	echo '#[allow(unused_variables)]' >> "$$ALLOW_TARGET"; \
	echo 'fn _k12_test() {}' >> "$$ALLOW_TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K12 gate MISSED #[allow] without WHY: — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K12 gate caught #[allow] without WHY:"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$ALLOW_TARGET.gate-bak" "$$ALLOW_TARGET"; \
	\
	echo "[R2] Injecting .ok() without logging..."; \
	R2_TARGET="$(PROJECT_DIR)/cynic-kernel/src/domain/mod.rs"; \
	cp "$$R2_TARGET" "$$R2_TARGET.gate-bak"; \
	printf '\n\n\n\n\n\n\n\n\n\nfn _r2_test() { std::fs::File::open("x").ok(); }\n' >> "$$R2_TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ R2 gate MISSED .ok() without logging — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R2 gate caught .ok() without logging"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$R2_TARGET.gate-bak" "$$R2_TARGET"; \
	\
	echo "[K3] Injecting pipeline fn in api/..."; \
	API_TARGET="$(PROJECT_DIR)/cynic-kernel/src/api/rest/mod.rs"; \
	cp "$$API_TARGET" "$$API_TARGET.gate-bak"; \
	echo 'fn format_crystal_context() {}' >> "$$API_TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K3 gate MISSED pipeline fn in api/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K3 gate caught pipeline fn in api/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$API_TARGET.gate-bak" "$$API_TARGET"; \
	\
	echo "[K4] Injecting duplicate Port trait name across domain/..."; \
	K4_TARGET1="$(PROJECT_DIR)/cynic-kernel/src/domain/mod.rs"; \
	K4_TARGET2="$(PROJECT_DIR)/cynic-kernel/src/domain/storage/mod.rs"; \
	cp "$$K4_TARGET1" "$$K4_TARGET1.gate-bak"; \
	cp "$$K4_TARGET2" "$$K4_TARGET2.gate-bak"; \
	echo 'trait _K4PhantomPort {}' >> "$$K4_TARGET1"; \
	echo 'trait _K4PhantomPort {}' >> "$$K4_TARGET2"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K4 gate MISSED duplicate Port trait — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K4 gate caught duplicate Port trait"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$K4_TARGET1.gate-bak" "$$K4_TARGET1"; \
	mv "$$K4_TARGET2.gate-bak" "$$K4_TARGET2"; \
	\
	echo "[R1] Injecting hardcoded /home/ path in scripts/..."; \
	R1_STUB="$(PROJECT_DIR)/scripts/_r1_gate_probe.sh"; \
	printf '%s\n' '#!/usr/bin/env bash' 'CYNIC=/home/user/cynic-hardcoded' > "$$R1_STUB"; \
	chmod +x "$$R1_STUB"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ R1 gate MISSED hardcoded /home/ path — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R1 gate caught hardcoded /home/ path"; PASS=$$((PASS+1)); \
	fi; \
	rm -f "$$R1_STUB"; \
	\
	echo ""; echo "── lint-drift ──"; \
	\
	echo "[Dormant] Injecting commented module without DORMANT tag..."; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo '// pub mod _gate_test_phantom;' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-drift >/dev/null 2>&1; then \
		echo "  ✗ Dormant gate MISSED commented module — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ Dormant gate caught commented module without DORMANT:"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[D4] Injecting phantom Dog in reference.md..."; \
	REF="$(PROJECT_DIR)/.claude/rules/reference.md"; \
	cp "$$REF" "$$REF.gate-bak"; \
	sed -i '/^| deterministic-dog/a | phantom-dog-test | Test | Nowhere |' "$$REF"; \
	if $(MAKE) --no-print-directory lint-drift >/dev/null 2>&1; then \
		echo "  ✗ D4 gate MISSED phantom Dog in reference.md — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ D4 gate caught phantom Dog in reference.md"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$REF.gate-bak" "$$REF"; \
	\
	echo "[K15] Injecting store_* with no read path in storage/mod.rs..."; \
	K15_TARGET="$(PROJECT_DIR)/cynic-kernel/src/domain/storage/mod.rs"; \
	cp "$$K15_TARGET" "$$K15_TARGET.gate-bak"; \
	printf '\n    async fn store_phantom_k15_gate_probe(&self) -> Result<(), StorageError> { unimplemented!() }\n' >> "$$K15_TARGET"; \
	if $(MAKE) --no-print-directory lint-drift >/dev/null 2>&1; then \
		echo "  ✗ K15 gate MISSED store_* without read path — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K15 gate caught store_* without read path"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$K15_TARGET.gate-bak" "$$K15_TARGET"; \
	\
	echo "[K17] Injecting StoragePort method not forwarded in ReconnectableStorage..."; \
	K17_TARGET="$(PROJECT_DIR)/cynic-kernel/src/domain/storage/mod.rs"; \
	cp "$$K17_TARGET" "$$K17_TARGET.gate-bak"; \
	sed -i '/^}$$/i\    async fn _k17_gate_probe(\&self) -> Result<(), StorageError> { Ok(()) }' "$$K17_TARGET"; \
	if $(MAKE) --no-print-directory lint-drift >/dev/null 2>&1; then \
		echo "  ✗ K17 gate MISSED missing ReconnectableStorage forward — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K17 gate caught missing ReconnectableStorage forward"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$K17_TARGET.gate-bak" "$$K17_TARGET"; \
	\
	echo ""; echo "── lint-subprocess-env ──"; \
	\
	echo "[R23a] Injecting shell script with cargo build and no RUST_MIN_STACK..."; \
	R23_STUB="$(PROJECT_DIR)/.claude/hooks/_r23_gate_probe.sh"; \
	printf '%s\n' '#!/usr/bin/env bash' '# R23 gate-test probe' 'cargo build --tests' > "$$R23_STUB"; \
	chmod +x "$$R23_STUB"; \
	if $(MAKE) --no-print-directory lint-subprocess-env >/dev/null 2>&1; then \
		echo "  ✗ R23a gate MISSED cargo without RUST_MIN_STACK — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R23a gate caught cargo without RUST_MIN_STACK"; PASS=$$((PASS+1)); \
	fi; \
	rm -f "$$R23_STUB"; \
	\
	echo "[R23b] Injecting shell script with standalone rustfmt and no --edition..."; \
	printf '%s\n' '#!/usr/bin/env bash' '# R23 gate-test probe' 'export RUST_MIN_STACK=67108864' 'rustfmt "$$FILE"' > "$$R23_STUB"; \
	chmod +x "$$R23_STUB"; \
	if $(MAKE) --no-print-directory lint-subprocess-env >/dev/null 2>&1; then \
		echo "  ✗ R23b gate MISSED rustfmt without --edition — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R23b gate caught rustfmt without --edition"; PASS=$$((PASS+1)); \
	fi; \
	rm -f "$$R23_STUB"; \
	\
	echo "[R23c] Injecting Rust Command::new(\"cargo\") without .env..."; \
	R23_RS_STUB="$(PROJECT_DIR)/cynic-kernel/src/_r23_gate_probe.rs"; \
	printf '%s\n' '// R23 gate-test probe' 'fn _r23_probe() {' '    tokio::process::Command::new("cargo").args(["build"]).spawn().ok();' '}' > "$$R23_RS_STUB"; \
	if $(MAKE) --no-print-directory lint-subprocess-env >/dev/null 2>&1; then \
		echo "  ✗ R23c gate MISSED Command::new(\"cargo\") without .env — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R23c gate caught Command::new(\"cargo\") without .env"; PASS=$$((PASS+1)); \
	fi; \
	rm -f "$$R23_RS_STUB"; \
	\
	echo ""; echo "── lint-security ──"; \
	\
	echo "[G1] Injecting OPEN CRITICAL finding..."; \
	TRACKER="$(PROJECT_DIR)/docs/audit/CYNIC-FINDINGS-TRACKER.md"; \
	if [ -f "$$TRACKER" ]; then \
		cp "$$TRACKER" "$$TRACKER.gate-bak"; \
		awk '/^## CRITICAL/{print; getline; print; getline; print; print "| GATE-TEST | Inject | Gate falsification test | OPEN |"; next} {print}' \
			"$$TRACKER.gate-bak" > "$$TRACKER"; \
		if $(MAKE) --no-print-directory lint-security >/dev/null 2>&1; then \
			echo "  ✗ G1 gate MISSED OPEN CRITICAL — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ G1 gate caught OPEN CRITICAL finding"; PASS=$$((PASS+1)); \
		fi; \
		mv "$$TRACKER.gate-bak" "$$TRACKER"; \
	else \
		echo "  SKIP: findings tracker not found"; \
	fi; \
	\
	echo ""; echo "── verify-hooks ──"; \
	\
	echo "[H1] Injecting wrong pre-push hook target..."; \
	TMPROOT=$$(mktemp -d); \
	mkdir -p "$$TMPROOT/project/scripts/git-hooks" "$$TMPROOT/git/hooks"; \
	cp "$(PROJECT_DIR)/scripts/git-hooks/pre-commit" "$$TMPROOT/project/scripts/git-hooks/pre-commit"; \
	cp "$(PROJECT_DIR)/scripts/git-hooks/pre-push" "$$TMPROOT/project/scripts/git-hooks/pre-push"; \
	chmod +x "$$TMPROOT/project/scripts/git-hooks/pre-commit" "$$TMPROOT/project/scripts/git-hooks/pre-push"; \
	ln -s "$$TMPROOT/project/scripts/git-hooks/pre-commit" "$$TMPROOT/git/hooks/pre-commit"; \
	ln -s /bin/true "$$TMPROOT/git/hooks/pre-push"; \
	if PROJECT_DIR_OVERRIDE="$$TMPROOT/project" GIT_DIR_OVERRIDE="$$TMPROOT/git" bash "$(PROJECT_DIR)/scripts/verify-hooks.sh" >/dev/null 2>&1; then \
		echo "  ✗ H1 gate MISSED wrong pre-push target — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ H1 gate caught wrong pre-push target"; PASS=$$((PASS+1)); \
	fi; \
	rm -rf "$$TMPROOT"; \
	\
	echo ""; echo "── verify-systemd-units ──"; \
	\
	echo "[UNIT1] Injecting wrong cynic-kernel.service target..."; \
	TMPROOT=$$(mktemp -d); \
	mkdir -p "$$TMPROOT/project/infra/systemd" "$$TMPROOT/systemd"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-kernel.service" "$$TMPROOT/project/infra/systemd/cynic-kernel.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/surrealdb.service" "$$TMPROOT/project/infra/systemd/surrealdb.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-healthcheck.service" "$$TMPROOT/project/infra/systemd/cynic-healthcheck.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-healthcheck.timer" "$$TMPROOT/project/infra/systemd/cynic-healthcheck.timer"; \
	cp "$(PROJECT_DIR)/infra/systemd/surrealdb-backup.service" "$$TMPROOT/project/infra/systemd/surrealdb-backup.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/surrealdb-backup.timer" "$$TMPROOT/project/infra/systemd/surrealdb-backup.timer"; \
	cp "$(PROJECT_DIR)/infra/systemd/llama-server.service" "$$TMPROOT/project/infra/systemd/llama-server.service"; \
	ln -s /bin/true "$$TMPROOT/systemd/cynic-kernel.service"; \
	ln -s "$$TMPROOT/project/infra/systemd/surrealdb.service" "$$TMPROOT/systemd/surrealdb.service"; \
	ln -s "$$TMPROOT/project/infra/systemd/cynic-healthcheck.service" "$$TMPROOT/systemd/cynic-healthcheck.service"; \
	ln -s "$$TMPROOT/project/infra/systemd/cynic-healthcheck.timer" "$$TMPROOT/systemd/cynic-healthcheck.timer"; \
	ln -s "$$TMPROOT/project/infra/systemd/surrealdb-backup.service" "$$TMPROOT/systemd/surrealdb-backup.service"; \
	ln -s "$$TMPROOT/project/infra/systemd/surrealdb-backup.timer" "$$TMPROOT/systemd/surrealdb-backup.timer"; \
	ln -s "$$TMPROOT/project/infra/systemd/llama-server.service" "$$TMPROOT/systemd/llama-server.service"; \
	if PROJECT_DIR_OVERRIDE="$$TMPROOT/project" SYSTEMD_USER_DIR_OVERRIDE="$$TMPROOT/systemd" bash "$(PROJECT_DIR)/scripts/verify-systemd-units.sh" >/dev/null 2>&1; then \
		echo "  ✗ UNIT1 gate MISSED wrong cynic-kernel.service target — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ UNIT1 gate caught wrong cynic-kernel.service target"; PASS=$$((PASS+1)); \
	fi; \
	rm -rf "$$TMPROOT"; \
	\
	echo ""; echo "── runtime-check ──"; \
	\
	echo "[RUNTIME1] Injecting duplicate llama-server process..."; \
	TMPROOT=$$(mktemp -d); \
	UNITDIR="$$TMPROOT/units"; BINDIR="$$TMPROOT/bin"; \
	mkdir -p "$$UNITDIR" "$$BINDIR"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-kernel.service" "$$UNITDIR/cynic-kernel.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/surrealdb.service" "$$UNITDIR/surrealdb.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/llama-server.service" "$$UNITDIR/llama-server.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-healthcheck.service" "$$UNITDIR/cynic-healthcheck.service"; \
	printf '%s\n' '#!/usr/bin/env bash' 'if [ "$$2" = "is-active" ]; then' '  case "$$3" in' '    cynic-kernel|surrealdb|llama-server) echo active ;;' '    *) echo inactive ;;' '  esac' '  exit 0' 'fi' 'if [ "$$2" = "show" ]; then' '  service="$$3"; prop="$${4#--property=}"' '  if [ "$$5" != "--value" ]; then exit 1; fi' '  case "$$service:$$prop" in' '    cynic-kernel:MainPID) echo 1001 ;;' '    cynic-kernel:SubState) echo running ;;' '    surrealdb:MainPID) echo 1002 ;;' '    surrealdb:SubState) echo running ;;' '    llama-server:MainPID) echo 1003 ;;' '    llama-server:SubState) echo running ;;' '    *) echo "" ;;' '  esac' '  exit 0' 'fi' 'exit 1' > "$$BINDIR/systemctl"; \
	printf '%s\n' '#!/usr/bin/env bash' 'if [ "$$1" = "-xo" ]; then' '  case "$$2" in' '    cynic-kernel) echo 1001 ;;' '    surreal) echo 1002 ;;' '    llama-server) echo 1003 ;;' '  esac' '  exit 0' 'fi' 'if [ "$$1" = "-xa" ]; then' '  case "$$2" in' '    cynic-kernel) echo "1001 cynic-kernel" ;;' '    surreal) echo "1002 surreal start" ;;' '    llama-server) printf "1003 llama-server\n1004 llama-server\n" ;;' '  esac' '  exit 0' 'fi' 'exit 1' > "$$BINDIR/pgrep"; \
	printf '%s\n' '#!/usr/bin/env bash' 'case "$$1" in' '  -o)' '    case "$$2" in' '      stat=) echo Ssl ;;' '      ppid=) echo 1 ;;' '    esac' '    ;;' 'esac' > "$$BINDIR/ps"; \
	printf '%s\n' '#!/usr/bin/env bash' 'cat <<OUT' 'LISTEN 0 128 127.0.0.1:3030 0.0.0.0:* users:(("cynic-kernel",pid=1001,fd=9))' 'LISTEN 0 128 127.0.0.1:8000 0.0.0.0:* users:(("surreal",pid=1002,fd=9))' 'LISTEN 0 128 127.0.0.1:8080 0.0.0.0:* users:(("llama-server",pid=1003,fd=9))' 'LISTEN 0 128 127.0.0.1:8080 0.0.0.0:* users:(("llama-server",pid=1004,fd=9))' 'OUT' > "$$BINDIR/ss"; \
	chmod +x "$$BINDIR/systemctl" "$$BINDIR/pgrep" "$$BINDIR/ps" "$$BINDIR/ss"; \
	if UNIT_DIR_OVERRIDE="$$UNITDIR" SYSTEMCTL_BIN_OVERRIDE="$$BINDIR/systemctl" PGREP_BIN_OVERRIDE="$$BINDIR/pgrep" PS_BIN_OVERRIDE="$$BINDIR/ps" SS_BIN_OVERRIDE="$$BINDIR/ss" bash "$(PROJECT_DIR)/scripts/runtime-truth.sh" check >/dev/null 2>&1; then \
		echo "  ✗ RUNTIME1 gate MISSED duplicate llama-server — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ RUNTIME1 gate caught duplicate llama-server"; PASS=$$((PASS+1)); \
	fi; \
	rm -rf "$$TMPROOT"; \
	\
	echo ""; \
	TOTAL=$$((PASS+FAIL)); \
	if [ $$FAIL -eq 0 ]; then \
		echo "✓ R21: $$PASS/$$TOTAL gates falsified successfully"; \
	else \
		echo "✗ R21: $$FAIL/$$TOTAL gates FAILED falsification"; exit 1; \
	fi

.PHONY: check-storage
check-storage: ## Integration tests against real SurrealDB (requires :8000)
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  CYNIC check-storage — integration tests"
	@echo "══════════════════════════════════════════"
	cargo test -p cynic-kernel --release --test integration_storage --test storage_contract
	@echo "✓ All checks passed"

# ── Stage 2: Validated Commit ────────────────────────────────
# Usage: make commit m="type(scope): description"
# Impact checking is done by [workspace.lints] in Cargo.toml — not bash scripts.
.PHONY: commit
commit: check
	@if [ -z "$(m)" ]; then echo "ERROR: provide message with m=\"...\"" >&2; exit 1; fi
	@echo ""
	@echo "▶ Staging and committing..."
	git add -u
	git commit -m "$(m)"
	@echo "✓ Committed (workspace lints + clippy + audit validated)"

# ── Stage 3: Ship (commit + push) ───────────────────────────
.PHONY: ship
ship: commit
	@echo ""
	@echo "▶ Pushing to origin..."
	git push origin $$(git rev-parse --abbrev-ref HEAD)
	@echo "✓ Shipped (pre-push validated build+test+clippy)"

# ── Stage 4: Deploy ─────────────────────────────────────────
# Integration tests run BEFORE deploy — if storage queries are broken, don't deploy.
.PHONY: deploy
deploy: ship check-storage
	@$(source_env)
	@echo ""
	@echo "▶ Backing up DB before deploy..."
	surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS:?Set SURREALDB_PASS in ~/.cynic-env}" \
		~/.surrealdb/backups/cynic_v2_pre_deploy_$$(date +%Y%m%d_%H%M%S).surql
	@echo "▶ Deploying kernel (saving previous for rollback)..."
	systemctl --user stop cynic-kernel
	@[ -f ~/bin/cynic-kernel ] && cp ~/bin/cynic-kernel ~/bin/cynic-kernel.prev || true
	cp $(PROJECT_DIR)/target/release/cynic-kernel ~/bin/cynic-kernel

	systemctl --user start cynic-kernel
	@echo "▶ Verifying (HTTP status code — no JSON parsing)..."
	@for i in $$(seq 1 15); do \
		sleep 3; \
		HTTP=$$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 \
			$${CYNIC_API_KEY:+-H "Authorization: Bearer $${CYNIC_API_KEY}"} \
			"http://$${CYNIC_REST_ADDR}/health" 2>/dev/null) && \
		[ "$$HTTP" = "200" ] && echo "  Kernel: sovereign (HTTP 200)" && break || \
		[ "$$HTTP" = "503" ] && echo "  Kernel: degraded (HTTP 503) — check Dogs" && break || \
		printf "."; \
	done
	@echo "▶ Healthcheck self-test..."
	@bash ~/bin/cynic-healthcheck.sh && echo "  Healthcheck: PASS" || echo "  ⚠ Healthcheck: FAIL — review ~/bin/cynic-healthcheck.sh"
	@echo "✓ Deployed and verified"

# ── Fast-path: Deploy existing binary without commit/push ────
# Use when origin/main is already current (e.g. after a PR merged through CI)
# and you just want to roll the release binary into ~/bin. Skips all validation
# gates — caller is responsible for freshness.
.PHONY: deploy-only
deploy-only:  ## Deploy target/release/cynic-kernel with no validation (skips check/commit/push)
	@$(source_env)
	@BIN="$(PROJECT_DIR)/target/release/cynic-kernel"; \
	[ -f "$$BIN" ] || { echo "✗ No release binary at $$BIN — run 'cargo build --release' or 'make check' first"; exit 1; }; \
	AGE_SEC=$$(( $$(date +%s) - $$(stat -c %Y "$$BIN") )); \
	if [ $$AGE_SEC -gt 3600 ]; then \
		echo "⚠ Binary is $$((AGE_SEC/60))min old — consider rebuild if source changed"; \
	fi
	@echo ""
	@echo "▶ deploy-only — no commit, no push, no validation"
	@echo "▶ Backing up DB before deploy..."
	surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS:?Set SURREALDB_PASS in ~/.cynic-env}" \
		~/.surrealdb/backups/cynic_v2_deploy_only_$$(date +%Y%m%d_%H%M%S).surql
	@echo "▶ Deploying kernel (saving previous for rollback)..."
	systemctl --user stop cynic-kernel
	@[ -f ~/bin/cynic-kernel ] && cp ~/bin/cynic-kernel ~/bin/cynic-kernel.prev || true
	cp $(PROJECT_DIR)/target/release/cynic-kernel ~/bin/cynic-kernel

	systemctl --user start cynic-kernel
	@echo "▶ Verifying (HTTP status code — no JSON parsing)..."
	@for i in $$(seq 1 15); do \
		sleep 3; \
		HTTP=$$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 \
			$${CYNIC_API_KEY:+-H "Authorization: Bearer $${CYNIC_API_KEY}"} \
			"http://$${CYNIC_REST_ADDR}/health" 2>/dev/null) && \
		[ "$$HTTP" = "200" ] && echo "  Kernel: sovereign (HTTP 200)" && break || \
		[ "$$HTTP" = "503" ] && echo "  Kernel: degraded (HTTP 503) — check Dogs" && break || \
		printf "."; \
	done
	@echo "▶ Healthcheck self-test..."
	@bash ~/bin/cynic-healthcheck.sh && echo "  Healthcheck: PASS" || echo "  ⚠ Healthcheck: FAIL — review ~/bin/cynic-healthcheck.sh"
	@echo "✓ Deployed (no commit, no push)"

# ── Emergency: Rollback to previous binary ─────────────────
.PHONY: rollback
rollback:
	@$(source_env)
	@[ -f ~/bin/cynic-kernel.prev ] || { echo "✗ No previous binary to rollback to"; exit 1; }
	@echo "▶ Rolling back to previous kernel binary..."
	systemctl --user stop cynic-kernel
	cp ~/bin/cynic-kernel.prev ~/bin/cynic-kernel

	systemctl --user start cynic-kernel
	@sleep 4
	@echo "▶ Verifying rollback..."
	@HTTP=$$(curl -s -o /dev/null -w '%{http_code}' "http://$${CYNIC_REST_ADDR}/health"); \
	[ "$$HTTP" = "200" ] && echo "Kernel: healthy (rolled back)" || echo "Kernel: DEGRADED (HTTP $$HTTP, rolled back)"

# ── Emergency: Hotfix deploy (skip push — for incidents) ──────
.PHONY: hotfix
hotfix: check
	@$(source_env)
	@echo ""
	@echo "⚠ HOTFIX DEPLOY — skipping push (commit locally, push later)"
	@echo "▶ Backing up DB..."
	surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS:?Set SURREALDB_PASS}" \
		~/.surrealdb/backups/cynic_v2_hotfix_$$(date +%Y%m%d_%H%M%S).surql
	@echo "▶ Deploying kernel..."
	systemctl --user stop cynic-kernel
	@[ -f ~/bin/cynic-kernel ] && cp ~/bin/cynic-kernel ~/bin/cynic-kernel.prev || true
	cp $(PROJECT_DIR)/target/release/cynic-kernel ~/bin/cynic-kernel

	systemctl --user start cynic-kernel
	@echo "▶ Verifying (retry loop)..."
	@for i in $$(seq 1 15); do \
		sleep 3; \
		HTTP=$$(curl -s -o /dev/null -w '%{http_code}' "http://$${CYNIC_REST_ADDR}/health" 2>/dev/null) && \
		[ "$$HTTP" = "200" ] && echo "Kernel: healthy (hotfix, HTTP $$HTTP)" && break || \
		printf "."; \
	done

# ── Restore DB from backup ────────────────────────────────────
.PHONY: restore
restore:
	@$(source_env)
	@if [ -z "$(F)" ]; then echo "ERROR: provide backup file with F=path"; echo "Available:"; ls -lhrt ~/.surrealdb/backups/ | tail -5; exit 1; fi
	@[ -f "$(F)" ] || { echo "ERROR: file $(F) not found"; exit 1; }
	@echo "⚠ RESTORING DB from $(F) — this will OVERWRITE current data"
	@read -p "Are you sure? (yes/no) " CONFIRM || true; [ "$$CONFIRM" = "yes" ] || { echo "Aborted"; exit 1; }
	systemctl --user stop cynic-kernel
	surreal import --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS}" "$(F)"
	systemctl --user start cynic-kernel
	@echo "✓ DB restored from $(F)"

.PHONY: test-restore
test-restore: ## Verify backup restore works against a test DB (non-destructive)
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  CYNIC test-restore — backup verification"
	@echo "══════════════════════════════════════════"
	@LATEST=$$(ls -t ~/.surrealdb/backups/*.surql.gz ~/.surrealdb/backups/*.surql 2>/dev/null | head -1); \
	if [ -z "$$LATEST" ]; then echo "✗ No backup files found"; exit 1; fi; \
	echo "▸ Testing restore of: $$LATEST"; \
	IMPORT_FILE="$$LATEST"; \
	if echo "$$LATEST" | grep -q '\.gz$$'; then \
		IMPORT_FILE=$$(mktemp /tmp/cynic-restore-test-XXXXXX.surql); \
		gunzip -c "$$LATEST" > "$$IMPORT_FILE"; \
	fi; \
	surreal import --endpoint http://localhost:8000 \
		--namespace cynic_test --database restore_test \
		--username root --password "$${SURREALDB_PASS}" \
		"$$IMPORT_FILE" 2>&1 | tail -3; \
	VCOUNT=$$(echo "USE NS cynic_test; USE DB restore_test; SELECT count() FROM verdict GROUP ALL;" | \
		surreal sql --endpoint http://localhost:8000 --username root --password "$${SURREALDB_PASS}" --hide-welcome 2>/dev/null | grep -oP '\d+' | head -1); \
	CCOUNT=$$(echo "USE NS cynic_test; USE DB restore_test; SELECT count() FROM crystal GROUP ALL;" | \
		surreal sql --endpoint http://localhost:8000 --username root --password "$${SURREALDB_PASS}" --hide-welcome 2>/dev/null | grep -oP '\d+' | head -1); \
	echo "▸ Verdicts: $${VCOUNT:-0}  Crystals: $${CCOUNT:-0}"; \
	echo "USE NS cynic_test; REMOVE DATABASE restore_test;" | \
		surreal sql --endpoint http://localhost:8000 --username root --password "$${SURREALDB_PASS}" --hide-welcome 2>/dev/null; \
	if echo "$$LATEST" | grep -q '\.gz$$'; then rm -f "$$IMPORT_FILE"; fi; \
	if [ "$${VCOUNT:-0}" -gt 0 ]; then echo "✓ Restore verified ($$VCOUNT verdicts, $${CCOUNT:-0} crystals)"; \
	else echo "⚠ Restore completed but 0 verdicts found — check backup contents"; fi

# ── Stage 5: Release (tag + changelog) ─────────────────────────
# Usage: make release v=patch  (or minor/major)
# Dry run: make release-dry v=patch
.PHONY: release
release: check
	@if [ -z "$(v)" ]; then echo "ERROR: provide version bump with v=patch|minor|major" >&2; exit 1; fi
	@echo ""
	@echo "▶ Releasing ($(v) bump)..."
	cargo release $(v) --execute --no-confirm
	@echo "✓ Released — changelog updated, tagged, pushed"

.PHONY: release-dry
release-dry:
	@if [ -z "$(v)" ]; then echo "ERROR: provide version bump with v=patch|minor|major" >&2; exit 1; fi
	cargo release $(v)

# ── Git hooks (Layer 2 enforcement) ──────────────────────────
.PHONY: install-hooks
install-hooks:
	@bash $(PROJECT_DIR)/scripts/install-hooks.sh

.PHONY: verify-hooks
verify-hooks:
	@bash $(PROJECT_DIR)/scripts/verify-hooks.sh

# ── Clean (fix SIGSEGV/LLVM crashes) ──────────────────────────
.PHONY: clean
clean:
	cargo clean -p cynic-kernel
	@echo "✓ Cleaned cynic-kernel build artifacts"

# ── Standalone: End-to-end test ──────────────────────────────
.PHONY: e2e
e2e:
	@$(source_env)
	@echo "▶ E2E: Sicilian Defense..."
	@ADDR="http://$${CYNIC_REST_ADDR}"; AUTH="Authorization: Bearer $${CYNIC_API_KEY}"; FAIL=0; \
	echo ""; \
	printf "  1/6 Health........... "; \
	HTTP=$$(curl -s -o /dev/null -w '%{http_code}' "$$ADDR/health"); \
	[ "$$HTTP" = "200" ] && echo "PASS ($$HTTP)" || { echo "FAIL ($$HTTP)"; FAIL=$$((FAIL+1)); }; \
	printf "  2/6 Judge+dog_scores. "; \
	V=$$(curl -s -X POST "$$ADDR/judge" -H "Content-Type: application/json" -H "$$AUTH" \
		-d '{"content":"1. e4 c5 — The Sicilian Defense.","context":"Most popular response to 1.e4","domain":"chess"}'); \
	DOGS=$$(echo "$$V" | python3 -c "import json,sys; v=json.load(sys.stdin); print(len(v.get('dog_scores',[])))" 2>/dev/null); \
	VERDICT=$$(echo "$$V" | python3 -c "import json,sys; print(json.load(sys.stdin)['verdict'])" 2>/dev/null); \
	[ "$${DOGS:-0}" -gt 0 ] && echo "PASS $$VERDICT dogs=$$DOGS" || { echo "FAIL (dog_scores empty)"; FAIL=$$((FAIL+1)); }; \
	printf "  3/6 Verdict DB RT.... "; \
	DB_DOGS=$$(curl -s -H "$$AUTH" "$$ADDR/verdicts?limit=1" | python3 -c "import json,sys; v=json.load(sys.stdin); print(len(v[0].get('dog_scores',[])) if v else 0)" 2>/dev/null); \
	[ "$${DB_DOGS:-0}" -gt 0 ] && echo "PASS ($$DB_DOGS dogs from DB)" || { echo "FAIL"; FAIL=$$((FAIL+1)); }; \
	printf "  4/6 Observations..... "; \
	OBS=$$(curl -s -H "$$AUTH" "$$ADDR/observations?limit=1" | python3 -c "import json,sys; o=json.load(sys.stdin); print(len(o[0].keys()) if o else 0)" 2>/dev/null); \
	[ "$${OBS:-0}" -ge 10 ] && echo "PASS ($$OBS fields)" || { echo "FAIL ($$OBS fields)"; FAIL=$$((FAIL+1)); }; \
	printf "  5/6 Audit............ "; \
	AUD=$$(curl -s -H "$$AUTH" "$$ADDR/audit?limit=1" | python3 -c "import json,sys; a=json.load(sys.stdin); print(len(a[0].keys()) if a else 0)" 2>/dev/null); \
	[ "$${AUD:-0}" -ge 5 ] && echo "PASS ($$AUD fields)" || { echo "FAIL ($$AUD fields)"; FAIL=$$((FAIL+1)); }; \
	printf "  6/6 Crystals......... "; \
	CRY=$$(curl -s -H "$$AUTH" "$$ADDR/crystals?limit=1" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))" 2>/dev/null); \
	[ "$${CRY:-0}" -gt 0 ] && echo "PASS" || { echo "FAIL"; FAIL=$$((FAIL+1)); }; \
	echo ""; \
	[ $$FAIL -eq 0 ] && echo "✓ All 6 E2E checks passed" || { echo "✗ $$FAIL E2E check(s) FAILED"; exit 1; }

# ── Standalone: System status ────────────────────────────────
.PHONY: status
status:
	@$(source_env)
	@echo "CYNIC System Status"
	@echo "═══════════════════"
	@printf "Kernel:  "; HTTP=$$(curl -s -o /dev/null -w '%{http_code}' "http://$${CYNIC_REST_ADDR}/health" 2>/dev/null); [ "$$HTTP" = "200" ] && echo "healthy ($$HTTP)" || echo "DEGRADED (HTTP $${HTTP:-000})"
	@printf "SurrealDB: "; surreal is-ready --endpoint http://localhost:8000 2>/dev/null && echo "ok" || echo "DOWN"
	@printf "Services: "; systemctl --user is-active cynic-kernel surrealdb llama-server 2>/dev/null | tr '\n' ' '; echo ""
	@printf "Hooks: "; $(MAKE) --no-print-directory verify-hooks >/dev/null 2>&1 && echo "verified" || echo "DRIFT"
	@printf "Units: "; $(MAKE) --no-print-directory verify-systemd-units >/dev/null 2>&1 && echo "verified" || echo "DRIFT"
	@printf "Runtime: "; $(MAKE) --no-print-directory runtime-check >/dev/null 2>&1 && echo "verified" || echo "DRIFT"
	@printf "Git: "; git -C $(PROJECT_DIR) rev-parse --abbrev-ref HEAD; git -C $(PROJECT_DIR) log --oneline -1
	@printf "Backup: "; ls -t ~/.surrealdb/backups/*.surql 2>/dev/null | head -1 || echo "none"

.PHONY: runtime-truth
runtime-truth:
	@$(source_env)
	@bash $(PROJECT_DIR)/scripts/runtime-truth.sh

.PHONY: runtime-check
runtime-check:
	@$(source_env)
	@$(MAKE) --no-print-directory verify-systemd-units
	@bash $(PROJECT_DIR)/scripts/runtime-truth.sh check

# ── Benchmark: Empirical evaluation against running kernel ───
.PHONY: benchmark
benchmark:
	@$(source_env)
	@python3 $(PROJECT_DIR)/scripts/benchmark.py --save

.PHONY: install-systemd-units
install-systemd-units:
	@bash $(PROJECT_DIR)/scripts/install-systemd-units.sh

.PHONY: verify-systemd-units
verify-systemd-units:
	@bash $(PROJECT_DIR)/scripts/verify-systemd-units.sh

# ── Standalone: Manual backup ────────────────────────────────
.PHONY: backup
backup:
	surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS:?Set SURREALDB_PASS in ~/.cynic-env}" \
		~/.surrealdb/backups/cynic_v2_$$(date +%Y%m%d_%H%M%S).surql
	@echo "✓ Backup saved"

# ── Scope Management ─────────────────────────────────────────
.PHONY: scope
scope:  ## make scope SLUG=rest-audit — create isolated worktree for an ILC
	@if [ -z "$(SLUG)" ]; then echo "ERROR: provide SLUG=<name> (e.g. make scope SLUG=rest-audit)" >&2; exit 1; fi
	@BRANCH="session/$$(id -un)/$(SLUG)"; \
	WORKTREE="$(PROJECT_DIR)/../cynic-$(SLUG)"; \
	git worktree add -b "$$BRANCH" "$$WORKTREE"; \
	echo ""; \
	echo "✓ Worktree: $$WORKTREE"; \
	echo "✓ Branch:   $$BRANCH"; \
	echo ""; \
	echo "Next steps:"; \
	echo "  1. cynic_coord_register(agent_id, intent)"; \
	echo "  2. cynic_coord_claim(agent_id, <target-file>)"; \
	echo "  3. cd $$WORKTREE && work"; \
	echo "  4. make check && cynic_coord_release(agent_id, <target-file>)"; \
	echo "  5. make done SLUG=$(SLUG) when finished"

.PHONY: done
done:  ## make done SLUG=rest-audit — remove worktree and branch
	@if [ -z "$(SLUG)" ]; then echo "ERROR: provide SLUG=<name>" >&2; exit 1; fi
	@BRANCH="session/$$(id -un)/$(SLUG)"; \
	WORKTREE="$(PROJECT_DIR)/../cynic-$(SLUG)"; \
	git worktree remove "$$WORKTREE" 2>/dev/null && echo "✓ Worktree removed" || echo "Note: worktree not found (may already be removed)"; \
	git branch -d "$$BRANCH" 2>/dev/null && echo "✓ Branch $$BRANCH deleted" || echo "Note: delete branch manually if merge is pending"; \
	echo "✓ Scope $(SLUG) released"

.PHONY: agents
agents:  ## Show active agents and work claims (requires kernel running)
	@$(source_env)
	@curl -sf -H "Authorization: Bearer $${CYNIC_API_KEY}" "http://$${CYNIC_REST_ADDR}/agents" | python3 -m json.tool 2>/dev/null || { [ -n "$${CYNIC_REST_ADDR:-}" ] && echo "ERROR: kernel not responding at $${CYNIC_REST_ADDR}" || echo "ERROR: CYNIC_REST_ADDR not set — source ~/.cynic-env first"; }

