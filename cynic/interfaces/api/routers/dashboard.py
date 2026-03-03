"""
CYNIC dashboard router " UI serving
"""
from __future__ import annotations

import logging
import pathlib as _pathlib

from fastapi import APIRouter, HTTPException
from fastapi.responses import FileResponse

logger = logging.getLogger("cynic.interfaces.api.server")

router_dashboard = APIRouter(tags=["dashboard"])

# Static dir " routers/ is one level deeper than api/, so parent.parent.parent
# reaches the package root (cynic/cynic/).
_static_dir = _pathlib.Path(__file__).parent.parent.parent / "static"


# "" Dashboard convenience route """"""""""""""""""""""""""""""""""""""""""""
@router_dashboard.get("/dashboard", include_in_schema=False)
async def dashboard() -> FileResponse:
    """Serve the live CYNIC kernel dashboard (connects to /ws/stream)."""
    path = _static_dir / "dashboard.html"
    if not path.is_file():
        raise HTTPException(status_code=404, detail="dashboard.html not found")
    return FileResponse(str(path), media_type="text/html")
