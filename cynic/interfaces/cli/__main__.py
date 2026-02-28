"""
CYNIC command-line interface entry point.

Enables running CYNIC as a module: python -m cynic

This delegates to the main() function in cynic.interfaces.cli which provides:
- Interactive CLI for organism control
- TUI dashboard for monitoring
- Configuration management

Typical usage:
    python -m cynic --help
    python -m cynic run
    python -m cynic dashboard

See Also:
    cynic.interfaces.cli.main: CLI implementation
"""
from cynic.interfaces.cli import main
main()
