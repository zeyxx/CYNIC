# organ-browser

`organ-browser` is the policy boundary for browser-capable agents.

Strategic base: `NousResearch/hermes-agent`.

Hermes Agent is the upstream bootstrap for browser-capable agent tooling:
browser tool surfaces, local/cloud backends, accessibility snapshots, CDP
supervision, MCP/tool registry patterns, and session search. CYNIC keeps policy,
coordination, evidence, memory, and human gates. See `HERMES_BOOTSTRAP.md`.

Native integration rule: Hermes is first-class only behind CYNIC contracts and
ports. Do not import Hermes internals into organ core logic. Observe upstream
capabilities through `organ-framework-map` / `organ-contracts`, execute through
CLI/gateway/ACP/MCP or `organ-browser` adapters, and normalize every result back
to CYNIC schemas.

Codex access should go through a `cynic-browser` MCP server. The server in
`src/entrypoints/mcp_server.py` is a minimal stdio MCP JSON-RPC server that
proxies the organ-browser Hub HTTP API, usually `http://127.0.0.1:40770`.

Current MCP tools are intentionally read-only:

- `browser_policy`
- `browser_status`
- `browser_tabs`
- `browser_render_public_url`

Target Hermes-compatible browser tools are broader, but policy-gated:

- `browser_navigate`
- `browser_snapshot`
- `browser_click`
- `browser_type`
- `browser_scroll`
- `browser_press`
- `browser_console`
- `browser_cdp`
- `browser_dialog`

The files in `contracts/` define the JSON task/result interface that
browser-capable agents must satisfy before CYNIC treats a browser observation as
real evidence.

Hard rules:

- no submit, post, sign, wallet, trade, stake, deposit, or auth mutation without
  an explicit human gate
- redact cookies, tokens, API keys, wallet secrets, personal Google data, and
  private account HTML
- every observation result must list evidence and mutations performed
- `mutations_performed` must be empty for read-only research tasks

Activation requirements:

- `organ-browser.service` running
- `organ-browser-hub.service` running
- `ORGAN_BROWSER_HUB_URL` set only when the hub is not on `127.0.0.1:40770`
- Codex configured with a `cynic-browser` MCP server and restarted, because MCP
  tools are loaded at session start

Operator activation:

```bash
python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --start-services
```

If that command reports `activation_status=partial`, inspect the failing checks.
In restricted Codex sandboxes, `.codex/config.toml`, `systemctl --user`, and
loopback access to `127.0.0.1:40770` may be unavailable even though the MCP
bridge itself is valid.

Validation:

```bash
python3 organs/browser/src/entrypoints/diagnose_browser_mcp.py
```

The bridge is only active for Codex when this diagnostic reports:

- `ok mcp_protocol`
- `ok codex_config_declares_browser`
- `ok browser_hub_http`
- `ok systemd_organ_browser_hub_active`

Hermes upstream capability scan, when a source checkout is available:

```bash
python3 organs/framework-map/src/entrypoints/cli.py map-hermes --source-root /path/to/hermes-agent
```
