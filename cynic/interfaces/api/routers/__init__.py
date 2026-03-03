"""
CYNIC API routers " REST endpoints for organism judgment and governance.

Organizes API routes by domain:
- act: Physical/blockchain actions
- actions: High-level behaviors
- chat: Conversational judgment interface
- consciousness: Consciousness state introspection
- governance: Governance and decision records
- health: System health and diagnostics
- observability: Metrics and observability
- organism: Organism lifecycle and control

Typical usage:
    from cynic.interfaces.api import app
    app.include_router(consciousness_router)

See Also:
    cynic.interfaces.api.handlers: Handler implementations for routes
    cynic.interfaces.api.entry: Main FastAPI application entry point
"""
