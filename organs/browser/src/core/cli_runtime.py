from __future__ import annotations

import argparse
import datetime
import json
from pathlib import Path

from organs.contracts.src.surface_contracts import read_json_file_or_default

from .entities import HumanGate, NavigationIntent
from .services import BrowserOrganService


def get_service() -> BrowserOrganService:
    from organs.browser.src.adapters.browser_hub_adapter import BrowserHubAdapter
    from organs.browser.src.adapters.filesystem_evidence_store import FilesystemEvidenceStore
    from organs.browser.src.adapters.firecrawl_adapter import FirecrawlAdapter
    from organs.browser.src.adapters.kernel_observation_adapter import CynicKernelObservationAdapter
    from organs.browser.src.adapters.searxng_adapter import SearXNGAdapter
    from organs.browser.src.adapters.secret_scanner_adapter import RegexSecretScannerAdapter

    return BrowserOrganService(
        search_port=SearXNGAdapter(),
        scrape_port=FirecrawlAdapter(),
        browser_port=BrowserHubAdapter(),
        evidence_store=FilesystemEvidenceStore(),
        secret_scanner=RegexSecretScannerAdapter(),
        kernel_obs=CynicKernelObservationAdapter(),
    )


def handle_search(args) -> None:
    service = get_service()
    results = service.search_port.search(args.query, limit=args.limit)
    print(json.dumps(results, indent=2))


def handle_scrape(args) -> None:
    service = get_service()
    result = service.scrape_port.scrape(args.url)
    sanitized_markdown = service.secret_scanner.sanitize(result.get("markdown", ""))
    print(sanitized_markdown)


def handle_interact(args) -> None:
    service = get_service()
    intent = NavigationIntent(
        id=f"manual-interact-{int(datetime.datetime.now().timestamp())}",
        created_at=datetime.datetime.now(),
        source="cli-manual",
        objective="Inspect URL content manually",
        url=args.url,
        allowed_actions=["scrape", "interact"],
        forbidden_actions=["submit", "post"],
        evidence_required=["page_markdown"],
        human_gate=HumanGate(required=False, reason=""),
    )
    evidence = service.execute_task(intent)
    print(f"Status: {evidence.status}")
    print(f"Summary: {evidence.summary}")
    if evidence.blockers:
        print(f"Blockers: {evidence.blockers}")


def handle_run_task(args) -> None:
    service = get_service()
    task_path = Path(args.task_file)
    if not task_path.exists():
        print(f"Error: Task file not found at {task_path}")
        raise SystemExit(1)

    task_data = read_json_file_or_default(task_path, {})

    created_at = datetime.datetime.now()
    if "created_at" in task_data:
        try:
            created_at = datetime.datetime.fromisoformat(task_data["created_at"].replace("Z", "+00:00"))
        except Exception:
            pass

    gate_data = task_data.get("human_gate", {"required": False, "reason": ""})

    intent = NavigationIntent(
        id=task_data["id"],
        created_at=created_at,
        source=task_data["source"],
        objective=task_data["objective"],
        url=task_data["url"],
        allowed_actions=task_data["allowed_actions"],
        forbidden_actions=task_data["forbidden_actions"],
        evidence_required=task_data["evidence_required"],
        human_gate=HumanGate(required=gate_data.get("required", False), reason=gate_data.get("reason", "")),
        context=task_data.get("context", {}),
    )

    evidence = service.execute_task(intent)
    result = {
        "task_id": evidence.task_id,
        "status": evidence.status,
        "observed_at": evidence.observed_at.isoformat(),
        "url": evidence.url,
        "summary": evidence.summary,
        "evidence": [
            {"kind": item.kind, "value": item.value, "redaction": item.redaction}
            for item in evidence.evidence
        ],
        "mutations_performed": evidence.mutations_performed,
        "blockers": evidence.blockers,
    }
    print(json.dumps(result, indent=2))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CYNIC Browser Organ CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)

    parser_search = subparsers.add_parser("search", help="Execute search discovery via SearXNG")
    parser_search.add_argument("query", help="The query to search for")
    parser_search.add_argument("--limit", type=int, default=5, help="Max results to return")
    parser_search.set_defaults(func=handle_search)

    parser_scrape = subparsers.add_parser("scrape", help="Extract markdown content from URL via Firecrawl/local fallback")
    parser_scrape.add_argument("url", help="The URL to scrape")
    parser_scrape.set_defaults(func=handle_scrape)

    parser_interact = subparsers.add_parser("interact", help="Perform browser interaction / CDP automation via Playwright")
    parser_interact.add_argument("url", help="The URL to interact with")
    parser_interact.set_defaults(func=handle_interact)

    parser_run = subparsers.add_parser("run-task", help="Execute browser task from JSON schema file")
    parser_run.add_argument("task_file", help="Path to JSON browser task file")
    parser_run.set_defaults(func=handle_run_task)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    args.func(args)
    return 0
