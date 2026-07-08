import argparse
import json
import os
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.append(str(Path(__file__).resolve().parents[2]))
    from src.adapters.kernel_log import KernelLogStateRootReader
    from src.adapters.observation_readback import JsonlObservationReadback
    from src.adapters.solana_anchor import MockSolanaAnchorAdapter
    from src.core.observation import observation_from_receipt
else:
    from ..adapters.kernel_log import KernelLogStateRootReader
    from ..adapters.observation_readback import JsonlObservationReadback
    from ..adapters.solana_anchor import MockSolanaAnchorAdapter
    from ..core.observation import observation_from_receipt


def _standard_log_path(project_root: Path) -> Path:
    return project_root / ".cynic" / "memory" / "logs" / "human-kernel.jsonl"


def _emit(payload: dict[str, object]) -> None:
    print(json.dumps(payload, ensure_ascii=False, sort_keys=True))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CYNIC Transparency Oracle (L1 Anchoring)")
    parser.add_argument("--action", choices=["anchor", "status", "observe-payload", "verify-observation"], default="anchor")
    parser.add_argument("--log-path", type=Path)
    parser.add_argument("--observation-log", type=Path)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    project_root = Path(os.environ.get("CYNIC_PROJECT_ROOT", os.getcwd()))
    log_path = args.log_path or _standard_log_path(project_root)
    reader = KernelLogStateRootReader(log_path)

    try:
        state_root = reader.latest()
    except FileNotFoundError as exc:
        _emit({"status": "error", "error": str(exc), "log_path": str(log_path)})
        return 1

    if args.action == 'status':
        _emit({"status": "ok", "state_root": state_root.to_dict()})
        return 0

    receipt = MockSolanaAnchorAdapter().anchor(state_root)
    if args.action == "observe-payload":
        _emit({"status": "ok", "observation": observation_from_receipt(receipt).to_dict()})
        return 0 if receipt.status == "anchored" else 1
    if args.action == "verify-observation":
        if args.observation_log is None:
            _emit({"status": "error", "error": "observation_log_required"})
            return 2
        visible = JsonlObservationReadback(args.observation_log).visible(observation_from_receipt(receipt))
        _emit({"status": "ok" if visible else "missing", "read_back": visible})
        return 0 if visible else 1
    _emit({"status": "ok", "receipt": receipt.to_dict()})
    return 0 if receipt.status == "anchored" else 1

if __name__ == "__main__":
    raise SystemExit(main())
