"""Pure source inventory model for external framework checkouts."""

from __future__ import annotations

from typing import Any


FRAMEWORK_SOURCE_INVENTORY_SCHEMA = "cynic.framework_source_inventory.v1"


def build_source_inventory_payload(
    *,
    framework: str,
    source_root: str,
    observed: dict[str, Any],
) -> dict[str, Any]:
    return {
        "schema": FRAMEWORK_SOURCE_INVENTORY_SCHEMA,
        "framework": framework,
        "source_root": source_root,
        "module_count": len(observed.get("python_modules", [])),
        "top_level_directory_count": len(observed.get("top_level_directories", [])),
        "cli_command_count": len(observed.get("cli_commands", [])),
        "toolset_count": len(observed.get("toolsets", [])),
        "tool_count": len(observed.get("tools", [])),
        "plugin_count": len(observed.get("plugins", [])),
        "skill_count": len(observed.get("skills", [])),
        "optional_mcp_count": len(observed.get("optional_mcps", [])),
        "provider_count": len(observed.get("providers", [])),
        "primitive_count": len(observed.get("primitives", [])),
        "layer_count": len(observed.get("layers", [])),
        "top_level_directories": observed.get("top_level_directories", []),
        "python_modules": observed.get("python_modules", []),
        "cli_commands": observed.get("cli_commands", []),
        "toolsets": observed.get("toolsets", []),
        "tools": observed.get("tools", []),
        "plugins": observed.get("plugins", []),
        "skills": observed.get("skills", []),
        "optional_mcps": observed.get("optional_mcps", []),
        "providers": observed.get("providers", []),
        "primitives": observed.get("primitives", []),
        "layers": observed.get("layers", []),
        "coverage_gaps": observed.get("coverage_gaps", []),
    }
