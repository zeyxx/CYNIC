"""File adapters for Hermes X curation artifacts."""

import json
import os
import shutil
from datetime import datetime
from pathlib import Path


def write_curated_files(curated_dir: Path, by_domain: dict[str, list[dict]]) -> None:
    """Write DomainSignal JSONL files to the organ-owned curation directory."""
    curated_dir.mkdir(parents=True, exist_ok=True)

    for domain, signals in by_domain.items():
        filepath = curated_dir / f"{domain}_curated.jsonl"
        with filepath.open("w", encoding="utf-8") as handle:
            for signal in signals:
                handle.write(json.dumps(signal) + "\n")
        print(f"Wrote {len(signals)} signals to {filepath}")


def sync_kernel_curation(
    curated_dir: Path,
    by_domain: dict[str, list[dict]],
    project_root: Path,
) -> None:
    """Sync curated files to the kernel curation directory when it exists."""
    root = os.environ.get("CYNIC_PROJECT_ROOT", str(project_root))
    kernel_curation_dir = Path(
        os.environ.get("CYNIC_CURATION_DIR", f"{root}/services/cynic-python/curation")
    )
    if not kernel_curation_dir.exists():
        print(f"Kernel curation dir not found: {kernel_curation_dir} - skipping sync")
        return

    synced = 0
    for domain in by_domain:
        src = curated_dir / f"{domain}_curated.jsonl"
        dst = kernel_curation_dir / f"{domain}_curated.jsonl"
        if src.exists():
            shutil.copy2(str(src), str(dst))
            synced += 1
    if synced > 0:
        print(f"Synced {synced} curated files to {kernel_curation_dir}")


def write_yield_summary(hermes_x_dir: Path, by_domain: dict[str, list[dict]]) -> None:
    """Write the curation yield summary consumed by search generation."""
    total = sum(len(signals) for signals in by_domain.values())
    yield_path = hermes_x_dir / "curation_yield.json"
    yield_data = {
        "timestamp": datetime.now().isoformat(),
        "total_tweets": total,
        "high_signal_tweets": total,
        "domains": {
            domain: {
                "signals": len(signals),
                "pct_of_total": round(len(signals) / max(total, 1) * 100, 1),
            }
            for domain, signals in by_domain.items()
        },
    }
    try:
        with yield_path.open("w", encoding="utf-8") as handle:
            json.dump(yield_data, handle, indent=2)
        print(f"Wrote curation yield to {yield_path}")
    except IOError as exc:
        print(f"Failed to write curation yield: {exc}")
