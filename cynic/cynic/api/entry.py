"""
CYNIC API Entry Point

Usage:
  python -m cynic.api.entry                    # defaults: 0.0.0.0:8765
  python -m cynic.api.entry --port 9000
  DATABASE_URL=postgresql://... python -m cynic.api.entry

Port 8765 = φ-derived: 8 (F(6)) × 765... just looks good.
Actually: floor(φ⁵ × 1000) = floor(11.09 × 1000) = 11090 → too big → 8765 (manual pick).
The real reason: it doesn't conflict with common ports (3000, 8000, 8080, 8888).
"""
import argparse
import logging
import os

import uvicorn

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s — %(message)s",
)

logger = logging.getLogger("cynic.api.entry")


def main() -> None:
    parser = argparse.ArgumentParser(description="CYNIC Kernel API Server")
    parser.add_argument("--host", default="0.0.0.0", help="Bind host")
    parser.add_argument("--port", type=int, default=int(os.getenv("PORT", "8765")), help="Bind port")
    parser.add_argument("--reload", action="store_true", help="Auto-reload on code changes (dev only)")
    parser.add_argument("--log-level", default="info", choices=["debug", "info", "warning", "error"])
    args = parser.parse_args()

    logger.info("*sniff* Starting CYNIC kernel on %s:%d", args.host, args.port)

    uvicorn.run(
        "cynic.api.server:app",
        host=args.host,
        port=args.port,
        reload=args.reload,
        log_level=args.log_level,
        access_log=True,
    )


if __name__ == "__main__":
    main()
