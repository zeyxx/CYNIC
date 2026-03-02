"""
CYNIC CLI -- `probes` command interface (Priority 10 Task 3).

Provides five commands for managing self-improvement proposals:
  1. list [--status PENDING|APPLIED|DISMISSED|all] — List proposals by status (default: PENDING)
  2. show <probe_id> — Show details of a single proposal
  3. approve <probe_id> — Approve and apply a proposal
  4. dismiss <probe_id> — Dismiss a proposal
  5. audit [--limit 50] — Show audit log of applied/dismissed proposals

All commands work with SelfProber and persist to ~/.cynic/self_proposals.json.
"""

from __future__ import annotations

import sys
from typing import Optional

from cynic.interfaces.cli.utils import _api_get, _c, _read_json, _CYNIC_DIR
from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
import os


# Dimension → color mapping
_PROBE_DIM_COLOR = {
    "QTABLE": "orange",
    "ESCORE": "yellow",
    "CONFIG": "cyan",
    "RESIDUAL": "dim",
    "METRICS": "green",
    "COUPLING": "red",
}


def _format_proposal_short(proposal) -> str:
    """Format a proposal for list view: [probe_id] DIMENSION — recommendation[:60]."""
    probe_id = proposal.probe_id[:8]
    dim = proposal.dimension
    rec = proposal.recommendation[:60]
    color = _PROBE_DIM_COLOR.get(dim, "white")
    sev = proposal.severity
    cur = proposal.current_value
    sug = proposal.suggested_value

    # Format: [probe_id] DIMENSION — recommendation
    #         severity: X.XX | current → suggested
    return (
        f"[{_c(color, probe_id)}] {_c(color, dim)} — {rec}\n"
        f"  severity: {sev:.2f} | {cur:.2f} → {sug:.2f}"
    )


def _format_proposal_full(proposal) -> str:
    """Format a proposal for show view with all fields."""
    lines = [
        f"Probe ID:      {proposal.probe_id}",
        f"Dimension:     {_c(_PROBE_DIM_COLOR.get(proposal.dimension, 'white'), proposal.dimension)}",
        f"Trigger:       {proposal.trigger}",
        f"Pattern Type:  {proposal.pattern_type}",
        f"Severity:      {proposal.severity:.2f}",
        f"Target:        {proposal.target}",
        f"Recommendation: {proposal.recommendation}",
        f"Current Value: {proposal.current_value:.4f}",
        f"Suggested:     {proposal.suggested_value:.4f}",
        f"Status:        {_c(_status_color(proposal.status), proposal.status)}",
        f"Proposed At:   {_format_timestamp(proposal.proposed_at)}",
    ]
    return "\n".join(lines)


def _status_color(status: str) -> str:
    """Return color for a status."""
    if status == "PENDING":
        return "yellow"
    elif status == "APPLIED":
        return "green"
    elif status == "DISMISSED":
        return "dim"
    return "white"


def _format_timestamp(timestamp: float) -> str:
    """Format Unix timestamp as human-readable string."""
    import datetime
    dt = datetime.datetime.fromtimestamp(timestamp)
    return dt.strftime("%Y-%m-%d %H:%M:%S")


def _format_audit_entry(proposal) -> str:
    """Format proposal for audit view: ✓ [probe_id] DIMENSION + recommendation[:60]."""
    status_icon = "✓" if proposal.status == "APPLIED" else "✗"
    probe_id = proposal.probe_id[:8]
    dim = proposal.dimension
    rec = proposal.recommendation[:60]
    color = _PROBE_DIM_COLOR.get(dim, "white")

    return f"{status_icon} [{_c(color, probe_id)}] {_c(color, dim)} — {rec}"


def cmd_probes_list(status: str = "PENDING") -> None:
    """List proposals by status filter.

    Args:
        status: PENDING (default) | APPLIED | DISMISSED | all
    """
    # Normalize status input
    status = status.upper()
    if status not in ("PENDING", "APPLIED", "DISMISSED", "ALL"):
        print(f"{_c('red', 'Error')}: Invalid status '{status}'. Use PENDING, APPLIED, DISMISSED, or all")
        sys.exit(1)

    # Try API first
    path = "/self-probes" + ("" if status == "PENDING" else f"?status={status.lower()}")
    api_data = _api_get(path)

    if api_data is not None:
        proposals = api_data.get("proposals", [])
        api_ok = True
    else:
        # File fallback
        prober = SelfProber()
        if status == "ALL":
            proposals = prober.all_proposals()
        elif status == "PENDING":
            proposals = prober.pending()
        else:
            proposals = [p for p in prober.all_proposals() if p.status == status]
        api_ok = False

    if not proposals:
        print(_c("dim", "No proposals found"))
        return

    mode = "API" if api_ok else "file"
    print(f"{_c('gray', f'({mode} mode)')} {status} proposals:\n")

    for proposal in proposals:
        # Convert dict to object if needed
        if isinstance(proposal, dict):
            from dataclasses import dataclass
            @dataclass
            class ProposalProxy:
                probe_id: str
                dimension: str
                recommendation: str
                severity: float
                current_value: float
                suggested_value: float
                status: str

            proposal = ProposalProxy(
                probe_id=proposal.get("probe_id", "?"),
                dimension=proposal.get("dimension", "?"),
                recommendation=proposal.get("recommendation", ""),
                severity=float(proposal.get("severity", 0.0)),
                current_value=float(proposal.get("current_value", 0.0)),
                suggested_value=float(proposal.get("suggested_value", 0.0)),
                status=proposal.get("status", "PENDING"),
            )

        print(_format_proposal_short(proposal))
        print()

    # Show stats
    if api_data and "stats" in api_data:
        stats = api_data["stats"]
        pending_str = _c('yellow', f"{stats['pending']} pending")
        applied_str = _c('green', f"{stats['applied']} applied")
        dismissed_str = _c('dim', f"{stats['dismissed']} dismissed")
        print(f"Stats: {pending_str}, {applied_str}, {dismissed_str}")
    elif not api_ok:
        stats = prober.stats()
        pending_str = _c('yellow', f"{stats['pending']} pending")
        applied_str = _c('green', f"{stats['applied']} applied")
        dismissed_str = _c('dim', f"{stats['dismissed']} dismissed")
        print(f"Stats: {pending_str}, {applied_str}, {dismissed_str}")


def cmd_probes_show(probe_id: str) -> None:
    """Show details of a single proposal.

    Args:
        probe_id: The proposal ID to display
    """
    prober = SelfProber()
    proposal = prober.get(probe_id)

    if proposal is None:
        print(f"{_c('red', 'Error')}: Proposal '{probe_id}' not found")
        sys.exit(1)

    print(_format_proposal_full(proposal))


def cmd_probes_approve(probe_id: str) -> None:
    """Approve and apply a proposal.

    Args:
        probe_id: The proposal ID to approve
    """
    prober = SelfProber()
    proposal = prober.apply(probe_id)

    if proposal is None:
        print(f"{_c('red', 'Error')}: Proposal '{probe_id}' not found")
        sys.exit(1)

    print(f"{_c('green', '✓ Approved')}: {proposal.probe_id}")
    print(f"  Recommendation: {proposal.recommendation[:80]}")
    print(f"  Change: {proposal.current_value:.4f} → {proposal.suggested_value:.4f}")


def cmd_probes_dismiss(probe_id: str) -> None:
    """Dismiss a proposal.

    Args:
        probe_id: The proposal ID to dismiss
    """
    prober = SelfProber()
    proposal = prober.dismiss(probe_id)

    if proposal is None:
        print(f"{_c('red', 'Error')}: Proposal '{probe_id}' not found")
        sys.exit(1)

    print(f"{_c('dim', '✗ Dismissed')}: {proposal.probe_id}")
    print(f"  Recommendation: {proposal.recommendation[:80]}")


def cmd_probes_audit(limit: int = 50) -> None:
    """Show audit log of applied/dismissed proposals.

    Args:
        limit: Maximum number of entries to show (default 50)
    """
    prober = SelfProber()
    proposals = prober.all_proposals()

    # Filter to APPLIED and DISMISSED, sort by proposed_at (newest first)
    audit = [p for p in proposals if p.status in ("APPLIED", "DISMISSED")]
    audit.sort(key=lambda p: p.proposed_at, reverse=True)
    audit = audit[:limit]

    if not audit:
        print(_c("dim", "No audit entries found"))
        return

    print(f"{_c('gray', f'Recent {len(audit)} actions (newest first):')} \n")

    for proposal in audit:
        print(_format_audit_entry(proposal))
        print(f"  {_c('gray', _format_timestamp(proposal.proposed_at))}")
        print()


def main() -> None:
    """Main entry point for probes CLI commands."""
    args = sys.argv[1:]

    if not args:
        # Default: show pending proposals
        cmd_probes_list("PENDING")
        return

    command = args[0].lower()

    if command == "list":
        status = "PENDING"  # default
        if "--status" in args:
            idx = args.index("--status")
            if idx + 1 < len(args):
                status = args[idx + 1].upper()
        cmd_probes_list(status)
    elif command == "show":
        if len(args) < 2:
            print(f"{_c('red', 'Error')}: show requires a probe_id")
            sys.exit(1)
        cmd_probes_show(args[1])
    elif command == "approve":
        if len(args) < 2:
            print(f"{_c('red', 'Error')}: approve requires a probe_id")
            sys.exit(1)
        cmd_probes_approve(args[1])
    elif command == "dismiss":
        if len(args) < 2:
            print(f"{_c('red', 'Error')}: dismiss requires a probe_id")
            sys.exit(1)
        cmd_probes_dismiss(args[1])
    elif command == "audit":
        limit = 50
        # Parse --limit option
        if len(args) > 1 and args[1] == "--limit" and len(args) > 2:
            try:
                limit = int(args[2])
            except ValueError:
                print(f"{_c('red', 'Error')}: --limit requires a numeric value")
                sys.exit(1)
        cmd_probes_audit(limit)
    elif command in ("help", "-h", "--help"):
        print("""CYNIC Probes CLI - Self-Improvement Proposal Management

Usage:
  cynic-probes list [--status PENDING|APPLIED|DISMISSED|all]  - List proposals (default: PENDING)
  cynic-probes show <probe_id>                                 - Show proposal details
  cynic-probes approve <probe_id>                              - Approve a proposal
  cynic-probes dismiss <probe_id>                              - Dismiss a proposal
  cynic-probes audit [--limit 50]                              - Show applied/dismissed audit log
  cynic-probes help                                            - Show this help

Examples:
  cynic-probes                        # Show pending proposals
  cynic-probes list --status all      # Show all proposals
  cynic-probes list --status applied  # Show applied proposals
  cynic-probes show abc12345          # Show details for proposal abc12345
  cynic-probes approve abc12345       # Mark proposal as applied
  cynic-probes audit --limit 100      # Show last 100 audit entries
""")
    else:
        print(f"{_c('red', 'Error')}: Unknown command '{command}'")
        print("Use 'cynic-probes help' for usage information")
        sys.exit(1)


if __name__ == "__main__":
    main()
