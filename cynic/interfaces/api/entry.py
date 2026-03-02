"""
CYNIC API Entry Point

Usage:
  python -m cynic.interfaces.api.entry                    # defaults: 0.0.0.0:8765
  python -m cynic.interfaces.api.entry --port 9000
  DATABASE_URL=postgresql://... python -m cynic.interfaces.api.entry

Port 8765 = Ï-derived: 8 (F(6)) Ã— 765... just looks good.
Actually: floor(Ïâµ Ã— 1000) = floor(11.09 Ã— 1000) = 11090 â’ too big â’ 8765 (manual pick).
The real reason: it doesn't conflict with common ports (3000, 8000, 8080, 8888).
"""
import argparse
import logging

import uvicorn

from cynic.kernel.core.config import CynicConfig

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s â€” %(message)s",
)

logger = logging.getLogger("cynic.interfaces.api.entry")


def main() -> None:
    _config = CynicConfig.from_env()

    # Validate configuration at startup (fail-fast for critical issues)
    issues = _config.validate()
    for issue in issues:
        if issue.startswith("CRITICAL:"):
            logger.error(issue)
        elif issue.startswith("WARN:"):
            logger.warning(issue)
        else:
            logger.info(issue)

    # Exit on critical production issues
    critical_issues = [i for i in issues if i.startswith("CRITICAL:")]
    is_prod = _config.environment not in ("development", "test", "local")
    if critical_issues and is_prod:
        logger.error("Cannot start in production with critical configuration issues. Exiting.")
        raise RuntimeError(f"Configuration validation failed: {critical_issues}")

    parser = argparse.ArgumentParser(description="CYNIC Kernel API Server")
    parser.add_argument("--host", default="0.0.0.0", help="Bind host")
    parser.add_argument("--port", type=int, default=_config.port, help="Bind port")
    parser.add_argument("--reload", action="store_true", help="Auto-reload on code changes (dev only)")
    parser.add_argument("--log-level", default="info", choices=["debug", "info", "warning", "error"])
    args = parser.parse_args()

    logger.info("*sniff* Starting CYNIC kernel on %s:%d", args.host, args.port)

    uvicorn.run(
        "cynic.interfaces.api.server:app",
        host=args.host,
        port=args.port,
        reload=args.reload,
        log_level=args.log_level,
        access_log=True,
    )


if __name__ == "__main__":
    main()
