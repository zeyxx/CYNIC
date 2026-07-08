# Codex MCP Activation

Canonical operator command:

```bash
python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --start-services
```

The organism-wide source of truth for this MCP exposure is
`infra/agent-exposure.json`, rendered by `organ-exposure`. To render the Codex
projection without applying it:

```bash
python3 organs/exposure/src/entrypoints/cli.py render --agent codex
```

This session could not edit `.codex/config.toml` because the sandbox exposes
that file as a read-only filesystem. If applying manually from a writable
operator session, add this block:

```toml
[mcp_servers.cynic-browser]
command = "python3"
args = ["./.cortex/mcp/run_browser_mcp.py"]

[mcp_servers.cynic-browser.tools.browser_policy]
approval_mode = "auto"

[mcp_servers.cynic-browser.tools.browser_status]
approval_mode = "auto"

[mcp_servers.cynic-browser.tools.browser_tabs]
approval_mode = "auto"

[mcp_servers.cynic-browser.tools.browser_task_state]
approval_mode = "auto"

[mcp_servers.cynic-browser.tools.browser_snapshot]
approval_mode = "auto"

[mcp_servers.cynic-browser.tools.browser_render_public_url]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_navigate]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_scroll]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_back]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_get_images]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_vision]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_console]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_cdp]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_dialog]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_press]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_click]
approval_mode = "approve"

[mcp_servers.cynic-browser.tools.browser_type]
approval_mode = "approve"
```

Then restart Codex. MCP tools are loaded at session start.

Runtime requirements if not using the activation script:

```bash
systemctl --user start organ-browser.service
systemctl --user start organ-browser-hub.service
curl http://127.0.0.1:40770/status
```

If Codex is sandboxed, it must be allowed to reach `127.0.0.1:40770`, or the
MCP server will load but report `Browser Hub is not reachable`.

Verify:

```bash
python3 organs/browser/src/entrypoints/diagnose_browser_mcp.py
```

Expected once fully activated:

```text
ok wrapper_exists
ok wrapper_executable
ok server_exists
ok python_mcp_import
ok server_import
ok mcp_protocol
ok codex_config_declares_browser
ok browser_hub_http
ok systemd_organ_browser_hub_active
```
