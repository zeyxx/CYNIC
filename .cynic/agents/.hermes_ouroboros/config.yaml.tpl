# Hermes Ouroboros Configuration (template)
# Source: .hermes_ouroboros/config.yaml.tpl
# Runtime: ~/.config/cynic/hermes-ouroboros-config.yaml
#
# This file uses <TAILSCALE_*> placeholders for IPs (substituted by fleet-gen.py)
# and ${VAR} placeholders for secrets (injected from ~/.cynic-env by systemd)

mcp_servers:
  cynic:
    command: /home/user/bin/cynic-kernel
    args:
      - --mcp
    env:
      CYNIC_API_KEY: "${CYNIC_API_KEY}"
      CYNIC_REST_ADDR: "<TAILSCALE_CORE>:3030"
      SURREALDB_PASS: "${SURREALDB_PASS}"
      SOVEREIGN_API_KEY: "${SOVEREIGN_API_KEY}"
      QWEN35_API_KEY: "${QWEN35_API_KEY}"
      HF_TOKEN: "${HF_TOKEN}"
      GEMMA_CORE_API_KEY: "${GEMMA_CORE_API_KEY}"
