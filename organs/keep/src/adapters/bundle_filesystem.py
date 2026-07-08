from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Any


class BundleFilesystemAdapter:
    def write_bundle(
        self,
        output_dir: Path,
        schema_path: Path,
        manifest: dict[str, Any],
        files: dict[str, str],
    ) -> dict[str, Any]:
        output_dir.mkdir(parents=True, exist_ok=True)
        for name, content in files.items():
            (output_dir / name).write_text(content, encoding="utf-8")
        if schema_path.exists():
            shutil.copyfile(schema_path, output_dir / "schema.surql")
        manifest["bundle_dir"] = str(output_dir)
        manifest["bundle_files"] = sorted([path.name for path in output_dir.iterdir()] + ["manifest.json"])
        (output_dir / "manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return manifest
