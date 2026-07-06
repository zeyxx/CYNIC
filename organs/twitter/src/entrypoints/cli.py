#!/usr/bin/env python3
"""Entrypoint for organ-twitter context ingestion and export."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from organs.twitter.src.adapters.twitter_cli_runtime import (
    bundle,
    diagnose,
    ingest,
    ingest_url,
    list_items,
    perceive,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="organ-twitter context ingestion and export")
    sub = parser.add_subparsers(dest="command")

    p_perceive = sub.add_parser("perceive", help="inspect X runtime and context state")
    p_perceive.add_argument("--no-save", dest="save", action="store_false")
    p_perceive.add_argument("--hermes-dataset", type=Path, default=None)
    p_perceive.set_defaults(func=perceive, save=True)

    p_ingest = sub.add_parser("ingest", help="append a tweet/article/note for cortex context")
    p_ingest.add_argument("--text", default="")
    p_ingest.add_argument("--url", default="")
    p_ingest.add_argument("--title", default="")
    p_ingest.add_argument("--author", default="")
    p_ingest.add_argument("--type", default="tweet", choices=["tweet", "thread", "article", "note"])
    p_ingest.add_argument("--source", default="manual")
    p_ingest.add_argument("--tag", action="append", default=[])
    p_ingest.add_argument("--notes", default="")
    p_ingest.add_argument("--item-id", default="")
    p_ingest.set_defaults(func=ingest)

    p_ingest_url = sub.add_parser("ingest-url", help="resolve and append an X/Twitter URL")
    p_ingest_url.add_argument("url")
    p_ingest_url.add_argument("--tag", action="append", default=[])
    p_ingest_url.add_argument("--hermes-dataset", type=Path, default=None)
    p_ingest_url.add_argument("--captures-dir", type=Path, default=None)
    p_ingest_url.add_argument("--bundle", action="store_true")
    p_ingest_url.add_argument("--limit", type=int, default=10)
    p_ingest_url.add_argument("--resolver-mode", choices=["auto", "browser-first", "local-first"], default="auto")
    p_ingest_url.set_defaults(func=ingest_url)

    p_list = sub.add_parser("list", help="list stored context items")
    p_list.add_argument("--query", default="")
    p_list.add_argument("--limit", type=int, default=20)
    p_list.set_defaults(func=list_items)

    p_bundle = sub.add_parser("bundle", help="export a cortex-readable context bundle")
    p_bundle.add_argument("--query", default="")
    p_bundle.add_argument("--limit", type=int, default=20)
    p_bundle.set_defaults(func=bundle)

    p_diagnose = sub.add_parser("diagnose", help="diagnose organ-twitter ingestion and browser access")
    p_diagnose.set_defaults(func=diagnose)
    return parser


def main():
    parser = build_parser()
    args = parser.parse_args()
    if not hasattr(args, "func"):
        args = parser.parse_args(["perceive"])
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
