#!/usr/bin/env python3
"""CLI membrane for organ-prospect."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

try:
    from organs.prospect.src.adapters.sovereign_search import ManualOnlySearchAdapter, SearxngSearchAdapter, SovereignSearchUnavailable
    from organs.prospect.src.adapters.website_auditor import WebsiteAuditor
    from organs.prospect.src.core.browser_tasks import build_browser_sourcing_task
    from organs.prospect.src.core.browser_results import read_browser_result, urls_from_browser_result
    from organs.prospect.src.core.prospecting import generate_prospects, generate_prospects_from_urls
except ImportError:
    ROOT = Path(__file__).resolve().parents[4]
    if str(ROOT) not in sys.path:
        sys.path.insert(0, str(ROOT))
    from organs.prospect.src.adapters.sovereign_search import ManualOnlySearchAdapter, SearxngSearchAdapter, SovereignSearchUnavailable
    from organs.prospect.src.adapters.website_auditor import WebsiteAuditor
    from organs.prospect.src.core.browser_tasks import build_browser_sourcing_task
    from organs.prospect.src.core.browser_results import read_browser_result, urls_from_browser_result
    from organs.prospect.src.core.prospecting import generate_prospects, generate_prospects_from_urls


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="organ-prospect local acquisition scanner")
    parser.add_argument("--category", required=True, help="Business category, e.g. couvreur")
    parser.add_argument("--city", required=True, help="City or area, e.g. Lyon")
    parser.add_argument("--max-results", type=int, default=25, help="Maximum prospects to emit")
    parser.add_argument("--input-url", action="append", default=[], help="Audit an explicit URL instead of relying only on search")
    parser.add_argument("--input-file", type=Path, help="File containing one URL per line")
    parser.add_argument("--browser-result", type=Path, help="Audit URLs extracted from an organ-browser result JSON")
    parser.add_argument(
        "--search-provider",
        choices=("manual", "searxng"),
        default="manual",
        help="Sovereign sourcing membrane. Default manual performs no web search.",
    )
    parser.add_argument("--searxng-url", help="Local CYNIC-controlled SearXNG base URL")
    parser.add_argument("--emit-browser-task", action="store_true", help="Emit an organ-browser task instead of auditing prospects")
    parser.add_argument("--browser-url", help="Optional starting URL for the read-only browser sourcing task")
    parser.add_argument("--task-id", help="Optional browser task id")
    parser.add_argument("--output", type=Path, help="Optional JSONL output path")
    parser.add_argument("--pretty", action="store_true", help="Print pretty JSON batch instead of JSONL")
    args = parser.parse_args(argv)

    if args.emit_browser_task:
        task = build_browser_sourcing_task(
            category=args.category,
            city=args.city,
            url=args.browser_url,
            task_id=args.task_id,
        )
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(task, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(task, ensure_ascii=False, indent=2 if args.pretty else None, sort_keys=not args.pretty))
        return 0

    input_urls = list(args.input_url)
    if args.browser_result:
        input_urls.extend(urls_from_browser_result(read_browser_result(args.browser_result)))
    if args.input_file:
        input_urls.extend(
            line.strip()
            for line in args.input_file.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.strip().startswith("#")
        )

    auditor = WebsiteAuditor()
    if input_urls:
        prospects = generate_prospects_from_urls(
            auditor=auditor,
            urls=input_urls[: args.max_results],
            category=args.category,
            city=args.city,
        )
    else:
        search = SearxngSearchAdapter(args.searxng_url) if args.search_provider == "searxng" else ManualOnlySearchAdapter()
        try:
            prospects = generate_prospects(
                search=search,
                auditor=auditor,
                category=args.category,
                city=args.city,
                max_results=args.max_results,
            )
        except SovereignSearchUnavailable as exc:
            parser.error(str(exc))
    rows = [prospect.to_json() for prospect in prospects]

    if args.output:
        write_jsonl(args.output, rows)

    if args.pretty:
        print(json.dumps({"schema": "cynic.prospect.batch.v1", "count": len(rows), "prospects": rows}, ensure_ascii=False, indent=2))
    else:
        for row in rows:
            print(json.dumps(row, ensure_ascii=False, sort_keys=True))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
