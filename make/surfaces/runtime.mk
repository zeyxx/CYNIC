PROJECT_DIR ?= $(shell git rev-parse --show-toplevel 2>/dev/null || pwd)
PROJECT_DIR := $(realpath $(PROJECT_DIR))

PYTHON_RUNTIME_ROOT := $(PROJECT_DIR)/.cynic/python-runtime
PYTHON_RUNTIME_BIN := $(PYTHON_RUNTIME_ROOT)/bin/python3
PYTHON_RUNTIME_PIP := $(PYTHON_RUNTIME_ROOT)/bin/pip3
PYTHON_RUNTIME_EXEC := $(if $(wildcard $(PYTHON_RUNTIME_BIN)),$(PYTHON_RUNTIME_BIN),python3)
RUNTIME_LOG_DIR := $(PYTHON_RUNTIME_ROOT)/logs

define ensure_runtime_venv
	@mkdir -p $(PYTHON_RUNTIME_ROOT) $(RUNTIME_LOG_DIR)
	@if [ ! -x "$(PYTHON_RUNTIME_BIN)" ]; then \
		if command -v uv >/dev/null 2>&1; then \
			echo "Creating organism venv via uv at $(PYTHON_RUNTIME_ROOT)"; \
			uv venv --seed $(PYTHON_RUNTIME_ROOT); \
		else \
			echo "Creating organism venv via python3 -m venv at $(PYTHON_RUNTIME_ROOT)"; \
			python3 -m venv --upgrade-deps $(PYTHON_RUNTIME_ROOT); \
		fi; \
	fi; \
	if [ -x "$(PYTHON_RUNTIME_BIN)" ] && [ ! -x "$(PYTHON_RUNTIME_PIP)" ]; then \
		$(PYTHON_RUNTIME_BIN) -m ensurepip --upgrade >"$(RUNTIME_LOG_DIR)/ensurepip.log" 2>&1 || true; \
	fi; \
	if [ -x "$(PYTHON_RUNTIME_PIP)" ]; then \
		$(PYTHON_RUNTIME_BIN) -m pip install --upgrade pip --disable-pip-version-check -q >"$(RUNTIME_LOG_DIR)/pip-upgrade.log" 2>&1 || true; \
		$(PYTHON_RUNTIME_BIN) -m pip install --upgrade uv --disable-pip-version-check -q >"$(RUNTIME_LOG_DIR)/uv-upgrade.log" 2>&1 || true; \
	fi
endef

.PHONY: python-runtime-bootstrap
python-runtime-bootstrap: ## Ensure an organism-level Python runtime is available at .cynic/python-runtime
	$(ensure_runtime_venv)
	@echo "runtime_venv=$(PYTHON_RUNTIME_ROOT)"

.PHONY: runtime-ready
runtime-ready: ## Bootstrap runtime + install latest hermes-agent for a frictionless reset path
	@$(MAKE) --no-print-directory python-runtime-bootstrap
	@$(MAKE) --no-print-directory hermes-update
	@$(MAKE) --no-print-directory python-runtime-status

.PHONY: hermes
hermes: ## Run hermes through the organism runtime: make hermes cmd="<args>"
	@if [ -z "$(cmd)" ]; then \
		echo "ERROR: cmd is required: make hermes cmd=\"<args>\"" >&2; \
		exit 2; \
	fi
	@$(MAKE) --no-print-directory hermes-run cmd="$(cmd)"

.PHONY: runtime-heal
runtime-heal: ## Reset organism runtime venv and re-bootstrap hermes (safe recovery)
	@rm -rf $(PYTHON_RUNTIME_ROOT)
	@$(MAKE) --no-print-directory runtime-ready

.PHONY: python-runtime-update
python-runtime-update: ## Upgrade a package in the organism-level runtime venv: make python-runtime-update tool=<name>
	@if [ -z "$(tool)" ]; then \
		echo "ERROR: tool is required: make python-runtime-update tool=<package>" >&2; \
		exit 2; \
	fi
	$(ensure_runtime_venv)
	@$(PYTHON_RUNTIME_BIN) -m pip install --upgrade $(tool)

.PHONY: hermes-update
hermes-update: ## Update Hermès CLI agent inside organism-level runtime
	$(ensure_runtime_venv)
	@$(PYTHON_RUNTIME_BIN) -m pip install --upgrade hermes-agent --disable-pip-version-check --quiet
	@if [ -x "$(PYTHON_RUNTIME_ROOT)/bin/hermes" ]; then \
		"$(PYTHON_RUNTIME_ROOT)/bin/hermes" --version; \
	else \
		echo "ERROR: hermes executable is missing in $(PYTHON_RUNTIME_ROOT)/bin"; \
		exit 2; \
	fi

.PHONY: hermes-run
hermes-run: ## Run hermes commands through organism-level runtime
	$(ensure_runtime_venv)
	@if [ -z "$(cmd)" ]; then \
		echo "ERROR: cmd is required: make hermes-run cmd=\"<args>\"" >&2; \
		exit 2; \
	fi
	@if [ -x "$(PYTHON_RUNTIME_ROOT)/bin/hermes" ]; then \
		"$(PYTHON_RUNTIME_ROOT)/bin/hermes" $(cmd); \
	else \
		echo "ERROR: hermes executable is missing in $(PYTHON_RUNTIME_ROOT)/bin" >&2; \
		exit 2; \
	fi

.PHONY: python-runtime-status
python-runtime-status: ## Show organism-level Python runtime status and installed command resolution
	$(ensure_runtime_venv)
	@echo "runtime_root=$(PYTHON_RUNTIME_ROOT)"
	@$(PYTHON_RUNTIME_BIN) --version
	@if [ -x "$(PYTHON_RUNTIME_PIP)" ]; then \
		$(PYTHON_RUNTIME_BIN) -m pip --version; \
	else \
		echo "pip: missing"; \
	fi
	@echo "hermes: $$(test -x $(PYTHON_RUNTIME_ROOT)/bin/hermes && printf '%s\\n' found || printf '%s\\n' missing)"
	@echo "cynic-runtime-path=$${PATH}"

runtime-principals-status: ## Non-mutating: inspect dedicated cynic runtime user and cortex launchers
	@$(PYTHON_RUNTIME_EXEC) $(PROJECT_DIR)/organs/supervisor/entrypoints/runtime_principals.py status

.PHONY: runtime-principals-plan
runtime-principals-plan: ## Non-mutating: show dedicated cynic runtime user activation plan
	@python3 $(PROJECT_DIR)/organs/supervisor/entrypoints/runtime_principals.py plan

.PHONY: runtime-principals-install-cynic-cli
runtime-principals-install-cynic-cli: ## Mutating: regenerate /usr/local/bin/cynic from the current project projection
	@python3 $(PROJECT_DIR)/organs/supervisor/entrypoints/runtime_principals.py print-cynic-cli | sudo tee /usr/local/bin/cynic >/dev/null
	@sudo chmod +x /usr/local/bin/cynic

.PHONY: runtime-principals-bootstrap
runtime-principals-bootstrap: ## Mutating: sync projection (update + apply + install-defaults) via cynic bootstrap
	@sudo python3 $(PROJECT_DIR)/organs/supervisor/entrypoints/runtime_principals.py bootstrap

.PHONY: runtime-principals-apply
runtime-principals-apply: ## Mutating: create cynic user and bounded cortex launchers; requires root
	@python3 $(PROJECT_DIR)/organs/supervisor/entrypoints/runtime_principals.py apply

.PHONY: runtime-principals-install-defaults
runtime-principals-install-defaults: ## Mutating: make interactive codex/agy shell commands route through cynic
	@python3 $(PROJECT_DIR)/organs/supervisor/entrypoints/runtime_principals.py install-defaults
