"""MCP tool metadata and policy builders for organ-browser."""

from __future__ import annotations

from typing import Any


HERMES_MUTATION_TOOLS = {"browser_click", "browser_type"}


def browser_policy_payload(*, hermes_capabilities: list[dict[str, Any]], hermes_quotas: dict[str, Any] | None, hub_url: str) -> dict[str, Any]:
    return {
        "mutation_default": "blocked",
        "allowed_without_human_gate": [
            "status",
            "list_tabs",
            "render_public_url",
            "extract_public_page_text",
        ],
        "forbidden_without_human_gate": [
            "submit_application",
            "post_to_social",
            "authenticate_personal_account",
            "connect_wallet",
            "sign_message_or_transaction",
            "trade",
            "stake",
            "deposit",
            "transfer_funds",
            "store_cookie_token_secret_or_private_profile_data",
        ],
        "hermes_tools": hermes_capabilities,
        "hermes_quotas": hermes_quotas,
        "secret_policy": "Never return cookies, bearer tokens, private keys, seed phrases, passwords, or personal account HTML.",
        "backend": "CYNIC Browser Hub HTTP API",
        "hub_url": hub_url,
    }


def hermes_tool_specs() -> dict[str, dict[str, Any]]:
    return {
        "browser_navigate": {
            "description": "Navigate to a public URL via Hermes-compatible tool pipeline.",
            "schema": {
                "type": "object",
                "required": ["url"],
                "additionalProperties": False,
                "properties": {
                    "url": {"type": "string", "description": "HTTP(S) URL"},
                    "task_id": {"type": "string"},
                    "human_gate": {"type": "boolean", "default": False},
                    "human_gate_reason": {"type": "string"},
                },
            },
        },
        "browser_snapshot": {
            "description": "Collect snapshot status and open tabs.",
            "schema": {
                "type": "object",
                "additionalProperties": False,
                "properties": {"task_id": {"type": "string"}},
            },
        },
        "browser_scroll": {
            "description": "Scroll viewport with Hermes-compatible semantics.",
            "schema": {
                "type": "object",
                "additionalProperties": False,
                "properties": {"task_id": {"type": "string"}, "delta": {"type": "number", "default": 0.5}},
            },
        },
        "browser_back": {
            "description": "Navigate back in browser history.",
            "schema": {
                "type": "object",
                "additionalProperties": False,
                "properties": {"task_id": {"type": "string"}},
            },
        },
        "browser_get_images": {
            "description": "Get current page images (Hermes-compatible placeholder surface).",
            "schema": {
                "type": "object",
                "additionalProperties": False,
                "properties": {"task_id": {"type": "string"}},
            },
        },
        "browser_vision": {
            "description": "Collect vision-related evidence from current page context.",
            "schema": {
                "type": "object",
                "additionalProperties": False,
                "properties": {"task_id": {"type": "string"}},
            },
        },
        "browser_console": {
            "description": "Inspect page console output.",
            "schema": {
                "type": "object",
                "additionalProperties": False,
                "properties": {"task_id": {"type": "string"}},
            },
        },
        "browser_cdp": {
            "description": "Run a raw CDP command (read-only by default).",
            "schema": {
                "type": "object",
                "required": ["command"],
                "additionalProperties": False,
                "properties": {"command": {"type": "string"}, "task_id": {"type": "string"}},
            },
        },
        "browser_dialog": {
            "description": "Handle dialog interactions.",
            "schema": {
                "type": "object",
                "required": ["action"],
                "additionalProperties": False,
                "properties": {"action": {"type": "string"}, "task_id": {"type": "string"}},
            },
        },
        "browser_click": {
            "description": "Click an element; requires explicit human gate via human_gate=true.",
            "schema": {
                "type": "object",
                "required": ["selector"],
                "additionalProperties": False,
                "properties": {
                    "selector": {"type": "string"},
                    "task_id": {"type": "string"},
                    "human_gate": {"type": "boolean", "default": False},
                    "human_gate_reason": {"type": "string"},
                },
            },
        },
        "browser_type": {
            "description": "Type text into element; requires explicit human gate via human_gate=true.",
            "schema": {
                "type": "object",
                "required": ["selector", "text"],
                "additionalProperties": False,
                "properties": {
                    "selector": {"type": "string"},
                    "text": {"type": "string"},
                    "task_id": {"type": "string"},
                    "human_gate": {"type": "boolean", "default": False},
                    "human_gate_reason": {"type": "string"},
                },
            },
        },
        "browser_press": {
            "description": "Press a key in page context.",
            "schema": {
                "type": "object",
                "required": ["key"],
                "additionalProperties": False,
                "properties": {
                    "key": {"type": "string"},
                    "task_id": {"type": "string"},
                    "human_gate": {"type": "boolean", "default": False},
                    "human_gate_reason": {"type": "string"},
                },
            },
        },
        "browser_task_state": {
            "description": "Read state and budget counters for a specific browser task/session.",
            "schema": {
                "type": "object",
                "required": ["task_id"],
                "additionalProperties": False,
                "properties": {"task_id": {"type": "string"}},
            },
        },
    }
