"""
Auto-Register Routers — Self-discovery and registration.

When CYNIC starts, it automatically finds and registers all routers.
No manual configuration needed.
"""
from __future__ import annotations

import logging
import importlib
import sys
from pathlib import Path
from typing import Any

from fastapi import FastAPI

logger = logging.getLogger(__name__)

ROUTERS_DIR = Path(__file__).parent


def auto_register_routers(app: FastAPI) -> dict[str, Any]:
    """
    Auto-discover and register all API routers.

    Scans cynic/api/routers/*.py for APIRouter instances.
    Registers them with FastAPI automatically.

    Looks for any variable matching pattern: router* (e.g., router, router_core, router_actions)

    Returns:
        Dict of registered routers with stats
    """
    registered = {}

    # Find all router modules
    router_files = sorted(ROUTERS_DIR.glob("*.py"))

    for router_file in router_files:
        # Skip __init__, auto_register, etc
        if router_file.name.startswith("_"):
            continue
        if router_file.name == "auto_register.py":
            continue

        module_name = router_file.stem

        try:
            # Import the module
            module_path = f"cynic.api.routers.{module_name}"
            if module_path in sys.modules:
                module = sys.modules[module_path]
            else:
                spec = importlib.util.spec_from_file_location(module_path, router_file)
                module = importlib.util.module_from_spec(spec)
                sys.modules[module_path] = module
                spec.loader.exec_module(module)

            # Look for APIRouter instance (pattern: router, router_*, etc)
            found_router = None
            for attr_name in dir(module):
                if "router" in attr_name.lower():
                    attr = getattr(module, attr_name)
                    # Check if it's an APIRouter
                    if hasattr(attr, "routes") and hasattr(attr, "include_router"):
                        found_router = attr
                        break

            if found_router:
                app.include_router(found_router)

                # Extract metadata
                tags = found_router.tags if hasattr(found_router, "tags") else []
                routes = len(found_router.routes) if hasattr(found_router, "routes") else 0

                registered[module_name] = {
                    "tags": tags,
                    "routes": routes,
                    "prefix": found_router.prefix if hasattr(found_router, "prefix") else "",
                }

                logger.info(
                    f"✓ Registered router: {module_name} "
                    f"({routes} routes, tags={tags})"
                )
            else:
                logger.debug(f"No APIRouter found in {module_name}")

        except Exception as e:
            logger.warning(f"Failed to register router {module_name}: {e}")

    logger.info(f"Auto-registered {len(registered)} routers: {list(registered.keys())}")
    return registered
