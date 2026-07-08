from __future__ import annotations

import asyncio
import functools
import importlib
import json
import os
import time
from pathlib import Path

import aiohttp
from aiohttp import web

from organs.browser.src.entrypoints._bootstrap import bootstrap_repo_path


VERSION = "0.1.0"
ROOT = bootstrap_repo_path()
DEFAULT_STATE_DIR = ROOT / "organs/browser/state"


def _state_dir() -> Path:
    return Path(os.environ.get("ORGAN_BROWSER_STATE_DIR", str(DEFAULT_STATE_DIR))).expanduser()


def _cdp_port() -> int:
    return int(os.environ.get("ORGAN_BROWSER_CDP_PORT", "40769"))


def _hub_port() -> int:
    return int(os.environ.get("ORGAN_BROWSER_HUB_PORT", "40770"))


async def _chrome_json(path: str) -> object:
    url = f"http://127.0.0.1:{_cdp_port()}{path}"
    async with aiohttp.ClientSession() as session:
        async with session.get(url, timeout=2) as response:
            return await response.json()


async def handle_status(_request: web.Request) -> web.Response:
    state_dir = _state_dir()
    status = {
        "organ": "organ-browser",
        "version": VERSION,
        "hub_port": _hub_port(),
        "cdp_port": _cdp_port(),
        "state_dir": str(state_dir),
        "started_at": _request.app["started_at"],
        "chrome": "unknown",
    }
    try:
        version = await _chrome_json("/json/version")
        status["chrome"] = "reachable"
        status["cdp"] = version
    except Exception as exc:
        status["chrome"] = "unreachable"
        status["blockers"] = [str(exc)]
    return web.json_response(status)


async def handle_tabs(_request: web.Request) -> web.Response:
    try:
        tabs = await _chrome_json("/json")
        return web.json_response(tabs)
    except Exception as exc:
        return web.json_response({"error": str(exc), "tabs": []}, status=503)


async def handle_render(request: web.Request) -> web.Response:
    url = request.query.get("url", "")
    if not url.startswith(("http://", "https://")):
        return web.json_response({"error": "missing or invalid ?url=http(s)://..."}, status=400)

    selector = request.query.get("selector")
    wait_ms = int(request.query.get("wait", "5000"))
    if wait_ms < 0 or wait_ms > 20000:
        return web.json_response({"error": "wait must be between 0 and 20000 ms"}, status=400)

    try:
        scripts_dir = str(ROOT / "scripts")
        import sys

        if scripts_dir not in sys.path:
            sys.path.insert(0, scripts_dir)
        web_render = importlib.import_module("web_render")
        loop = asyncio.get_event_loop()
        result = await loop.run_in_executor(
            None,
            functools.partial(web_render.render_url, url, selector=selector, wait_ms=wait_ms),
        )
        return web.json_response(result)
    except Exception as exc:
        return web.json_response({"error": str(exc), "url": url}, status=500)


def create_app() -> web.Application:
    app = web.Application()
    app["started_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    app.router.add_get("/status", handle_status)
    app.router.add_get("/tabs", handle_tabs)
    app.router.add_get("/render", handle_render)
    return app


async def main_async() -> None:
    state_dir = _state_dir()
    state_dir.mkdir(parents=True, exist_ok=True)
    state_file = state_dir / "browser-state.json"
    state_file.write_text(
        json.dumps(
            {
                "organ": "organ-browser",
                "hub_port": _hub_port(),
                "cdp_port": _cdp_port(),
                "hub_url": f"http://127.0.0.1:{_hub_port()}",
                "started_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )

    runner = web.AppRunner(create_app())
    await runner.setup()
    site = web.TCPSite(runner, "127.0.0.1", _hub_port())
    await site.start()
    while True:
        await asyncio.sleep(3600)


def main() -> int:
    asyncio.run(main_async())
    return 0
