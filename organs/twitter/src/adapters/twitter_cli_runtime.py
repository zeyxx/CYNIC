"""Runtime handlers for the organ-twitter CLI."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from urllib.error import URLError
from urllib.request import urlopen

from organs.contracts.src.project_root import resolve_repo_root
from organs.twitter.src.adapters.context_exporter import ContextBundleExporter
from organs.twitter.src.adapters.context_repository import JsonlContextRepository
from organs.twitter.src.adapters.twitter_perceiver import JsonStateStorage, SystemTwitterPerceiver
from organs.twitter.src.adapters.x_resolver import (
    AuthenticatedBrowserCdpArticleResolver,
    BrowserHubRenderResolver,
    CompositeSocialSourceResolver,
    LocalHermesDatasetResolver,
    PublicEmbedResolver,
    RawCaptureResolver,
)
from organs.twitter.src.core.entities import SocialContextItem
from organs.twitter.src.core.use_cases import (
    ExportContextBundle,
    IngestContextItem,
    IngestSocialUrl,
    ListContextItems,
    PerceiveTwitterSurface,
)


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

STATE_DIR = Path(os.environ.get("ORGAN_TWITTER_STATE_DIR", PROJECT_ROOT / "organs" / "twitter" / "state"))
STATE_FILE = STATE_DIR / "state.json"
CONTEXT_ITEMS = STATE_DIR / "context_items.jsonl"
CONTEXT_DIR = STATE_DIR / "context"
DEFAULT_HERMES_DATASET = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl"
DEFAULT_HERMES_CAPTURES = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "captures"


def read_stdin() -> str:
    if sys.stdin.isatty():
        return ""
    return sys.stdin.read().strip()


def print_json(payload: dict) -> None:
    print(json.dumps(payload, indent=2, ensure_ascii=False))


def perceive(args: argparse.Namespace) -> int:
    perceiver = SystemTwitterPerceiver(STATE_DIR, hermes_dataset=args.hermes_dataset)
    storage = JsonStateStorage(STATE_FILE)
    state = PerceiveTwitterSurface(perceiver, storage).execute(save=args.save)
    print_json(
        {
            "status": "ok",
            "state_file": str(STATE_FILE),
            "state": {
                "total_tweets_captured": state.total_tweets_captured,
                "total_verdicts_judged": state.total_verdicts_judged,
                "total_context_items": state.total_context_items,
                "latest_context_bundle": state.latest_context_bundle,
                "proxy_status": state.proxy_status,
                "timestamp": state.timestamp,
            },
        }
    )
    return 0


def ingest(args: argparse.Namespace) -> int:
    text = args.text or read_stdin()
    if not text:
        print("organ-twitter ingest requires --text or stdin content", file=sys.stderr)
        return 2

    item = SocialContextItem(
        item_id=args.item_id or "",
        source=args.source,
        source_type=args.type,
        text=text,
        url=args.url or "",
        title=args.title or "",
        author=args.author or "",
        tags=args.tag or [],
        notes=args.notes or "",
        metadata={"ingested_by": "organ-twitter-cli"},
    )
    stored = IngestContextItem(JsonlContextRepository(CONTEXT_ITEMS)).execute(item)
    print_json({"status": "ok", "item": stored.__dict__, "dataset": str(CONTEXT_ITEMS)})
    return 0


def ingest_url(args: argparse.Namespace) -> int:
    dataset = args.hermes_dataset or DEFAULT_HERMES_DATASET
    local_resolvers = [LocalHermesDatasetResolver(dataset), RawCaptureResolver(args.captures_dir or DEFAULT_HERMES_CAPTURES)]
    browser_resolvers = [AuthenticatedBrowserCdpArticleResolver(), BrowserHubRenderResolver()]
    public_resolvers = [PublicEmbedResolver()]
    if args.resolver_mode == "browser-first":
        resolvers = browser_resolvers + local_resolvers + public_resolvers
    elif args.resolver_mode == "local-first":
        resolvers = local_resolvers + browser_resolvers + public_resolvers
    else:
        resolvers = [AuthenticatedBrowserCdpArticleResolver()] + local_resolvers + [BrowserHubRenderResolver()] + public_resolvers

    stored = IngestSocialUrl(CompositeSocialSourceResolver(resolvers), JsonlContextRepository(CONTEXT_ITEMS)).execute(
        args.url,
        tags=args.tag,
    )
    print_json(
        {
            "status": "ok",
            "item": stored.__dict__,
            "dataset": str(CONTEXT_ITEMS),
            "hermes_dataset": str(dataset),
        }
    )
    if args.bundle:
        context_bundle, paths = ExportContextBundle(
            JsonlContextRepository(CONTEXT_ITEMS),
            ContextBundleExporter(CONTEXT_DIR),
        ).execute(query=stored.author or stored.title or stored.item_id, limit=args.limit)
        print_json({"status": "bundled", "bundle_id": context_bundle.bundle_id, "items": len(context_bundle.items), "paths": paths})
    return 0


def list_items(args: argparse.Namespace) -> int:
    items = ListContextItems(JsonlContextRepository(CONTEXT_ITEMS)).execute(query=args.query, limit=args.limit)
    print_json({"status": "ok", "count": len(items), "items": [item.__dict__ for item in items]})
    return 0


def bundle(args: argparse.Namespace) -> int:
    context_bundle, paths = ExportContextBundle(
        JsonlContextRepository(CONTEXT_ITEMS),
        ContextBundleExporter(CONTEXT_DIR),
    ).execute(query=args.query or "", limit=args.limit)
    print_json({"status": "ok", "bundle_id": context_bundle.bundle_id, "items": len(context_bundle.items), "paths": paths})
    return 0


def diagnose(args: argparse.Namespace) -> int:
    hub_url = os.environ.get("ORGAN_BROWSER_HUB_URL", "http://127.0.0.1:40770").rstrip("/")
    browser_status = {"reachable": False, "error": ""}
    try:
        with urlopen(f"{hub_url}/status", timeout=2) as response:
            browser_status = {
                "reachable": True,
                "status": response.status,
                "body": response.read(500).decode("utf-8", errors="replace"),
            }
    except (OSError, URLError, TimeoutError) as exc:
        browser_status = {"reachable": False, "error": str(exc)}

    codex_config = PROJECT_ROOT / ".codex" / "config.toml"
    codex_text = codex_config.read_text(encoding="utf-8") if codex_config.exists() else ""
    capture_count = len(list(DEFAULT_HERMES_CAPTURES.glob("*.json"))) if DEFAULT_HERMES_CAPTURES.exists() else 0
    payload = {
        "status": "ok",
        "state_dir": str(STATE_DIR),
        "context_items": {"path": str(CONTEXT_ITEMS), "count": JsonlContextRepository(CONTEXT_ITEMS).count_items()},
        "latest_bundle": {
            "markdown": str(CONTEXT_DIR / "latest.md"),
            "json": str(CONTEXT_DIR / "latest.json"),
            "exists": (CONTEXT_DIR / "latest.json").exists(),
        },
        "hermes_dataset": {
            "path": str(DEFAULT_HERMES_DATASET),
            "exists": DEFAULT_HERMES_DATASET.exists(),
            "size": DEFAULT_HERMES_DATASET.stat().st_size if DEFAULT_HERMES_DATASET.exists() else 0,
        },
        "raw_captures": {"path": str(DEFAULT_HERMES_CAPTURES), "exists": DEFAULT_HERMES_CAPTURES.exists(), "json_files": capture_count},
        "browser_hub": {"url": hub_url, **browser_status},
        "codex_browser_mcp": {
            "config": str(codex_config),
            "declared": "mcp_servers.cynic-browser" in codex_text,
            "sandbox_full_access": 'sandbox_mode = "full-access"' in codex_text
            or 'sandbox_mode = "danger-full-access"' in codex_text,
        },
        "recommended_next_action": (
            "Restart Codex after applying organ-exposure render for cynic-browser and full-access sandbox."
            if not browser_status["reachable"]
            else "Browser Hub reachable; use ingest-url --bundle for one-command X context ingestion."
        ),
    }
    print_json(payload)
    return 0
