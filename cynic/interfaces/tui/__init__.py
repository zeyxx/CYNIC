"""
CYNIC TUI — Terminal user interface for organism monitoring and control.

Provides interactive terminal dashboard for:
- Real-time consciousness level and activity monitoring
- Proposal viewing and voting participation
- System health and metrics visualization
- Dog judgment details and confidence scores
- Learning loop progress tracking

Architecture:
    panels: Individual dashboard panels (consciousness, proposals, metrics, etc.)
    layout: Screen layout management and panel composition
    commands: Interactive command handlers for user input

Typical usage:
    from cynic.interfaces.cli import run_tui
    await run_tui()  # Start interactive dashboard

See Also:
    cynic.interfaces.cli: CLI infrastructure and main entry point
    cynic.interfaces.api.routers.dashboard: Web-based dashboard alternative
"""
