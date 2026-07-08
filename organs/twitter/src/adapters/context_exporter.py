import dataclasses
import json
from pathlib import Path

from ..core.entities import ContextBundle
from ..core.ports import IContextExporter


class ContextBundleExporter(IContextExporter):
    def __init__(self, output_dir: Path):
        self.output_dir = output_dir

    def export_bundle(self, bundle: ContextBundle) -> dict[str, str]:
        self.output_dir.mkdir(parents=True, exist_ok=True)
        json_path = self.output_dir / f"{bundle.bundle_id}.json"
        md_path = self.output_dir / f"{bundle.bundle_id}.md"
        latest_json = self.output_dir / "latest.json"
        latest_md = self.output_dir / "latest.md"

        payload = dataclasses.asdict(bundle)
        markdown = bundle.to_markdown()
        for path in (json_path, latest_json):
            with open(path, "w", encoding="utf-8") as handle:
                json.dump(payload, handle, indent=2, ensure_ascii=False)
        for path in (md_path, latest_md):
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(markdown)
        return {
            "json": str(json_path),
            "markdown": str(md_path),
            "latest_json": str(latest_json),
            "latest_markdown": str(latest_md),
        }
