#!/usr/bin/env python3
"""Command dispatch primitives for runtime-principals.

This module keeps the command surface declarative: each command is represented as a
spec object and executed through a dispatcher. It is intentionally small so tests
and cortex operators can consume a stable command manifest while handlers remain
swappable.
"""

from __future__ import annotations

from typing import Any

from organs.contracts.src.surface_contracts import CommandSurfaceSpec, SurfaceRegistry


RuntimePrincipalsCommandSpec = CommandSurfaceSpec


class RuntimePrincipalsCommandDispatcher:
    """Runtime-principals command dispatcher with explicit spec ownership."""

    def __init__(self, specs: list[RuntimePrincipalsCommandSpec]):
        self._specs: dict[str, RuntimePrincipalsCommandSpec] = {}
        for spec in specs:
            self._specs[spec.name] = spec

    @property
    def command_names(self) -> list[str]:
        return sorted(self._specs)

    def spec(self, name: str) -> RuntimePrincipalsCommandSpec:
        return self._specs[name]

    def manifest(self) -> dict[str, Any]:
        specs = sorted(self._specs.values(), key=lambda item: item.name)
        return {
            "schema": "cynic.runtime_principals.command_manifest.v1",
            "count": len(specs),
            "commands": [spec.as_manifest() for spec in specs],
        }

    def surface_registry(self) -> SurfaceRegistry:
        return SurfaceRegistry(commands=tuple(self._specs.values()))

    def registry_manifest(self) -> dict[str, Any]:
        return self.surface_registry().manifest()

    def execute(self, command: str, args: list[str]) -> int:
        spec = self._specs[command]
        return spec.handler(args)
