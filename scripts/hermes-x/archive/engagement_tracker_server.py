#!/usr/bin/env python3
"""
CYNIC Hermes Engagement Tracker Server

Simple HTTP server that receives X.com engagement events from browser extension
and logs them to engagement.jsonl for correlation with agent predictions.

Usage:
    python3 engagement_tracker_server.py --organ-dir ~/.cynic/organs/hermes/x \
                                         --port 8888

Install browser extension:
  1. chrome://extensions/ → "Load unpacked"
  2. Point to ~/.cynic/organs/hermes/x-engagement-extension/
  3. Start this server
  4. Browse X.com normally — engagement events logged silently
"""

__version__ = "0.1.0"

import json
import logging
from datetime import datetime
from pathlib import Path
from typing import Optional
import asyncio
from aiohttp import web

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("engagement-tracker")


class EngagementTracker:
    """Receive and log engagement events."""

    def __init__(self, organ_dir: Path):
        self.organ_dir = Path(organ_dir)
        self.output_file = self.organ_dir / "engagement.jsonl"
        self.event_count = 0

    async def handle_engagement(self, request) -> web.Response:
        """Handle POST request from browser extension."""
        try:
            data = await request.json()
        except Exception as e:
            logger.error("Invalid JSON: %s", e)
            return web.Response(status=400, text="Invalid JSON")

        # Validate required fields
        if not data.get("tweet_id"):
            return web.Response(status=400, text="Missing tweet_id")

        if not data.get("action"):
            return web.Response(status=400, text="Missing action")

        # Log engagement event
        event = {
            "tweet_id": data["tweet_id"],
            "action": data["action"],  # 'like', 'bookmark', 'reply', 'retweet', 'follow'
            "author": data.get("author", "unknown"),
            "text_preview": data.get("text_preview", ""),
            "timestamp": data.get("timestamp", datetime.now().isoformat()),
            "url": data.get("url", "")
        }

        try:
            with open(self.output_file, "a") as f:
                f.write(json.dumps(event) + "\n")

            self.event_count += 1

            logger.info(
                "[%d] %s @%s: %s",
                self.event_count,
                event["action"].upper(),
                event["author"],
                event["tweet_id"]
            )

            return web.Response(status=200, text="OK")

        except Exception as e:
            logger.error("Failed to log engagement: %s", e)
            return web.Response(status=500, text="Server error")

    async def handle_health(self, request) -> web.Response:
        """Health check endpoint."""
        return web.json_response({
            "status": "ok",
            "version": __version__,
            "events_logged": self.event_count,
            "output_file": str(self.output_file)
        })

    async def handle_stats(self, request) -> web.Response:
        """Return current statistics."""
        # Count events by action
        action_counts = {
            "like": 0,
            "bookmark": 0,
            "reply": 0,
            "retweet": 0,
            "follow": 0,
            "total": 0
        }

        if self.output_file.exists():
            with open(self.output_file) as f:
                for line in f:
                    try:
                        event = json.loads(line)
                        action = event.get("action", "unknown")
                        if action in action_counts:
                            action_counts[action] += 1
                        action_counts["total"] += 1
                    except json.JSONDecodeError:
                        pass

        return web.json_response({
            "timestamp": datetime.now().isoformat(),
            "engagement_counts": action_counts,
            "file_exists": self.output_file.exists()
        })

    def create_app(self) -> web.Application:
        """Create aiohttp application."""
        app = web.Application()
        app.router.add_post("/log-engagement", self.handle_engagement)
        app.router.add_get("/health", self.handle_health)
        app.router.add_get("/stats", self.handle_stats)

        # CORS middleware
        @web.middleware
        async def cors_middleware(request, handler):
            response = await handler(request)
            response.headers["Access-Control-Allow-Origin"] = "*"
            response.headers["Access-Control-Allow-Methods"] = "POST, GET, OPTIONS"
            response.headers["Access-Control-Allow-Headers"] = "Content-Type"
            return response

        app.middlewares.append(cors_middleware)

        # Handle OPTIONS requests
        async def handle_options(request):
            return web.Response(status=200)

        app.router.add_options("/log-engagement", handle_options)
        app.router.add_options("/stats", handle_options)

        return app


async def main():
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Hermes Engagement Tracker")
    parser.add_argument("--organ-dir", type=Path, default=Path.home() / ".cynic/organs/hermes/x")
    parser.add_argument("--port", type=int, default=8888)
    parser.add_argument("--host", default="localhost")
    args = parser.parse_args()

    organ_dir = args.organ_dir.expanduser()
    if not organ_dir.exists():
        logger.error("Organ directory not found: %s", organ_dir)
        return 1

    logger.info("Engagement Tracker v%s", __version__)
    logger.info("Listening on http://%s:%d", args.host, args.port)
    logger.info("Output file: %s/engagement.jsonl", organ_dir)

    tracker = EngagementTracker(organ_dir)
    app = tracker.create_app()

    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, args.host, args.port)
    await site.start()

    logger.info("✓ Server started. Press Ctrl+C to stop.")

    try:
        # Keep running
        while True:
            await asyncio.sleep(3600)
    except KeyboardInterrupt:
        logger.info("Shutting down...")
        await runner.cleanup()
        return 0


if __name__ == "__main__":
    exit(asyncio.run(main()))
