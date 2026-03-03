"""CLI Views for rendering observability data in the terminal.

Provides rendering functions for displaying:
- OBSERVE view: All three streams (CYNIC, Human, Machine) together
- CYNIC view: Deep dive into CYNIC's observations, thinking, and planning
- MACHINE view: Machine resource utilization and health

Uses ASCII progress bars (â-ˆ for filled, â-' for empty) and emojis for clarity.
"""

from __future__ import annotations

from cynic.kernel.observability.models import SymbioticState


def _render_progress_bar(
    value: float,
    max_value: float,
    width: int = 20,
) -> str:
    """Render a progress bar using ASCII characters.

    Args:
        value: Current value (0 to max_value).
        max_value: Maximum value for scaling.
        width: Width of the bar in characters.

    Returns:
        str: ASCII progress bar like "â-ˆâ-ˆâ-ˆâ-ˆâ-'â-'â-'â-'â-'â-'â-'â-'â-'â-'â-'â-'â-'â-'â-'â-'"
    """
    normalized = min(max(value / max_value, 0.0), 1.0)
    filled = int(normalized * width)
    empty = width - filled
    return "â-ˆ" * filled + "â-'" * empty


def _format_percentage(value: float, precision: int = 1) -> str:
    """Format a value as a percentage.

    Args:
        value: The value to format.
        precision: Number of decimal places.

    Returns:
        str: Formatted percentage string like "45.2%"
    """
    return f"{value:.{precision}f}%"


def render_observe_view(state: SymbioticState) -> str:
    """Render OBSERVE view showing all three streams together.

    Displays:
    - CYNIC section (confidence, e-score, thinking, planning)
    - YOUR STATE section (energy/focus bars, intentions, values)
    - MACHINE CAPACITY (CPU/RAM/Disk bars with %)
    - SYMBIOTIC ALIGNMENT section

    Args:
        state: Current SymbioticState snapshot.

    Returns:
        str: Formatted view as a multi-line string.
    """
    lines = []

    # Header
    lines.append("\n" + "=" * 70)
    lines.append(" ðŸ'ï¸  OBSERVE - Current State Snapshot")
    lines.append("=" * 70)

    # CYNIC section
    lines.append("\nðŸ§  CYNIC")
    lines.append("-" * 70)
    lines.append(
        f"  Confidence:  {_render_progress_bar(state.cynic_confidence, 1.0, 15)} "
        f"{_format_percentage(state.cynic_confidence * 100)}"
    )
    lines.append(
        f"  E-Score:     {_render_progress_bar(state.cynic_e_score, 1.0, 15)} "
        f"{state.cynic_e_score:.3f}"
    )
    lines.append(f"  Thinking:    {state.cynic_thinking}")
    lines.append("  Planning:")
    for item in state.cynic_planning:
        lines.append(f"    â€¢ {item}")

    # YOUR STATE section
    lines.append("\nðŸ¤ YOUR STATE")
    lines.append("-" * 70)
    lines.append(
        f"  Energy:      {_render_progress_bar(state.human_energy, 10.0, 15)} "
        f"{state.human_energy:.1f}/10.0"
    )
    lines.append(
        f"  Focus:       {_render_progress_bar(state.human_focus, 10.0, 15)} "
        f"{state.human_focus:.1f}/10.0"
    )
    if state.human_intentions:
        lines.append("  Intentions:")
        for intent in state.human_intentions:
            lines.append(f"    â€¢ {intent}")
    else:
        lines.append("  Intentions:  (none)")
    if state.human_values:
        lines.append("  Values:")
        for val in state.human_values:
            lines.append(f"    â€¢ {val}")

    # MACHINE CAPACITY section
    lines.append("\nâš™ï¸  MACHINE CAPACITY")
    lines.append("-" * 70)
    cpu = state.machine_resources.get("cpu_percent", 0.0)
    memory = state.machine_resources.get("memory_percent", 0.0)
    disk = state.machine_resources.get("disk_percent", 0.0)

    lines.append(
        f"  CPU:         {_render_progress_bar(cpu, 100.0, 15)} " f"{_format_percentage(cpu)}"
    )
    lines.append(
        f"  Memory:      {_render_progress_bar(memory, 100.0, 15)} " f"{_format_percentage(memory)}"
    )
    lines.append(
        f"  Disk:        {_render_progress_bar(disk, 100.0, 15)} " f"{_format_percentage(disk)}"
    )

    # SYMBIOTIC ALIGNMENT section
    lines.append("\nðŸ"- SYMBIOTIC ALIGNMENT")
    lines.append("-" * 70)
    lines.append(
        f"  Alignment:   {_render_progress_bar(state.alignment_score, 1.0, 15)} "
        f"{_format_percentage(state.alignment_score * 100)}"
    )

    if state.conflicts:
        lines.append("  Conflicts:")
        for conflict in state.conflicts:
            lines.append(f"    âš ï¸  {conflict}")
    else:
        lines.append("  Conflicts:   (none)")

    if state.shared_objectives:
        lines.append("  Shared Objectives:")
        for obj in state.shared_objectives:
            lines.append(f"    âœ" {obj}")

    lines.append("\n" + "=" * 70)

    return "\n".join(lines)


def render_cynic_view(state: SymbioticState) -> str:
    """Render CYNIC deep dive view.

    Displays detailed information about CYNIC's observations, thinking,
    and planning items.

    Args:
        state: Current SymbioticState snapshot.

    Returns:
        str: Formatted view as a multi-line string.
    """
    lines = []

    # Header
    lines.append("\n" + "=" * 70)
    lines.append(" ðŸ'­ CYNIC MIND - Deep Dive")
    lines.append("=" * 70)

    # Observations
    lines.append("\nðŸ"‹ OBSERVATIONS")
    lines.append("-" * 70)
    if state.cynic_observations:
        for key, value in state.cynic_observations.items():
            lines.append(f"  {key:.<40} {value}")
    else:
        lines.append("  (no observations)")

    # Current Thinking
    lines.append("\nðŸ¤" CURRENT THINKING")
    lines.append("-" * 70)
    lines.append(f"  {state.cynic_thinking}")

    # Confidence
    lines.append("\n  Confidence Level:")
    lines.append(
        f"    {_render_progress_bar(state.cynic_confidence, 1.0, 30)} "
        f"{_format_percentage(state.cynic_confidence * 100)}"
    )

    # Planning
    lines.append("\nðŸ"‹ PLANNING ITEMS")
    lines.append("-" * 70)
    if state.cynic_planning:
        for idx, item in enumerate(state.cynic_planning, 1):
            lines.append(f"  {idx}. {item}")
    else:
        lines.append("  (no planning items)")

    # E-Score
    lines.append("\nâš¡ E-SCORE (Energy/Consciousness)")
    lines.append("-" * 70)
    lines.append(f"  Score:  {state.cynic_e_score:.3f}")
    lines.append(f"  Bar:    {_render_progress_bar(state.cynic_e_score, 1.0, 40)}")

    lines.append("\n" + "=" * 70)

    return "\n".join(lines)


def render_machine_view(state: SymbioticState) -> str:
    """Render MACHINE view showing resource utilization and health.

    Displays CPU, memory, and disk usage with progress bars,
    health indicators, and constraint warnings.

    Args:
        state: Current SymbioticState snapshot.

    Returns:
        str: Formatted view as a multi-line string.
    """
    lines = []

    # Header
    lines.append("\n" + "=" * 70)
    lines.append(" âš™ï¸  MACHINE - Resources & Health")
    lines.append("=" * 70)

    # Resource percentages section
    lines.append("\nðŸ"Š RESOURCE UTILIZATION")
    lines.append("-" * 70)

    cpu = state.machine_resources.get("cpu_percent", 0.0)
    memory = state.machine_resources.get("memory_percent", 0.0)
    disk = state.machine_resources.get("disk_percent", 0.0)
    temp = state.machine_resources.get("temperature", 0.0)
    network = state.machine_resources.get("network_bandwidth", 0.0)

    lines.append("  CPU:")
    lines.append(f"    {_render_progress_bar(cpu, 100.0, 30)} {_format_percentage(cpu)}")

    lines.append("  Memory:")
    lines.append(f"    {_render_progress_bar(memory, 100.0, 30)} {_format_percentage(memory)}")

    lines.append("  Disk:")
    lines.append(f"    {_render_progress_bar(disk, 100.0, 30)} {_format_percentage(disk)}")

    if temp > 0:
        lines.append("  Temperature:")
        lines.append(f"    {temp:.1f}Â°C")

    if network > 0:
        lines.append("  Network Bandwidth:")
        lines.append(f"    {network:.0f} bytes/sec")

    # Health indicators
    lines.append("\nâœ… HEALTH INDICATORS")
    lines.append("-" * 70)

    is_healthy = state.machine_health.get("is_healthy", False)
    health_icon = "âœ"" if is_healthy else "âœ-"
    lines.append(f"  Overall Health:    {health_icon}")

    cpu_ok = state.machine_health.get("cpu_ok", False)
    cpu_icon = "âœ"" if cpu_ok else "âœ-"
    lines.append(f"  CPU OK:            {cpu_icon}")

    memory_ok = state.machine_health.get("memory_ok", False)
    memory_icon = "âœ"" if memory_ok else "âœ-"
    lines.append(f"  Memory OK:         {memory_icon}")

    disk_ok = state.machine_health.get("disk_ok", False)
    disk_icon = "âœ"" if disk_ok else "âœ-"
    lines.append(f"  Disk OK:           {disk_icon}")

    # Constraints and warnings
    constraints = state.machine_constraints.get("warnings", [])
    if constraints:
        lines.append("\nâš ï¸  CONSTRAINTS & WARNINGS")
        lines.append("-" * 70)
        for constraint in constraints:
            lines.append(f"  â€¢ {constraint}")
    else:
        lines.append("\nâš ï¸  CONSTRAINTS & WARNINGS")
        lines.append("-" * 70)
        lines.append("  (no warnings)")

    # Capability delta
    if state.machine_capability_delta:
        lines.append("\nðŸ"" CAPABILITY CHANGES")
        lines.append("-" * 70)
        for delta in state.machine_capability_delta:
            lines.append(f"  â€¢ {delta}")

    lines.append("\n" + "=" * 70)

    return "\n".join(lines)
