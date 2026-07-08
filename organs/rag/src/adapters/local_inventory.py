"""Local filesystem inventory adapter for organ-rag."""

from __future__ import annotations

from pathlib import Path


class LocalRagInventory:
    def __init__(self, organ_root: Path) -> None:
        self.organ_root = organ_root

    def files(self) -> list[str]:
        return [
            str(path.relative_to(self.organ_root))
            for path in sorted(self.organ_root.rglob("*"))
            if path.is_file() and ".git" not in path.parts
        ]

