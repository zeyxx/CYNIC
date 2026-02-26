"""CLI Views for rendering observability data in the terminal.

Provides rendering functions for displaying:
- OBSERVE view: All three streams (CYNIC, Human, Machine) together
- CYNIC view: Deep dive into CYNIC's observations, thinking, and planning
- MACHINE view: Machine resource utilization and health

Uses ASCII progress bars (█ for filled, ░ for empty) and emojis for clarity.
"""

from __future__ import annotations

from cynic.observability.models import SymbioticState


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
        str: ASCII progress bar like "████░░░░░░░░░░░░░░░░"
    """
    normalized = min(max(value / max_value, 0.0), 1.0)
    filled = int(normalized * width)
    empty = width - filled
    return "█" * filled + "░" * empty


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
    lines.append(" 👁️  OBSERVE - Current State Snapshot")
    lines.append("=" * 70)

    # CYNIC section
    lines.append("\n🧠 CYNIC")
    lines.append("-" * 70)
    lines.append(f"  Confidence:  {_render_progress_bar(state.cynic_confidence, 1.0, 15)} "
                 f"{_format_percentage(state.cynic_confidence * 100)}")
    lines.append(f"  E-Score:     {_render_progress_bar(state.cynic_e_score, 1.0, 15)} "
                 f"{state.cynic_e_score:.3f}")
    lines.append(f"  Thinking:    {state.cynic_thinking}")
    lines.append(f"  Planning:")
    for item in state.cynic_planning:
        lines.append(f"    • {item}")

    # YOUR STATE section
    lines.append("\n🤝 YOUR STATE")
    lines.append("-" * 70)
    lines.append(f"  Energy:      {_render_progress_bar(state.human_energy, 10.0, 15)} "
                 f"{state.human_energy:.1f}/10.0")
    lines.append(f"  Focus:       {_render_progress_bar(state.human_focus, 10.0, 15)} "
                 f"{state.human_focus:.1f}/10.0")
    if state.human_intentions:
        lines.append(f"  Intentions:")
        for intent in state.human_intentions:
            lines.append(f"    • {intent}")
    else:
        lines.append(f"  Intentions:  (none)")
    if state.human_values:
        lines.append(f"  Values:")
        for val in state.human_values:
            lines.append(f"    • {val}")

    # MACHINE CAPACITY section
    lines.append("\n⚙️  MACHINE CAPACITY")
    lines.append("-" * 70)
    cpu = state.machine_resources.get('cpu_percent', 0.0)
    memory = state.machine_resources.get('memory_percent', 0.0)
    disk = state.machine_resources.get('disk_percent', 0.0)

    lines.append(f"  CPU:         {_render_progress_bar(cpu, 100.0, 15)} "
                 f"{_format_percentage(cpu)}")
    lines.append(f"  Memory:      {_render_progress_bar(memory, 100.0, 15)} "
                 f"{_format_percentage(memory)}")
    lines.append(f"  Disk:        {_render_progress_bar(disk, 100.0, 15)} "
                 f"{_format_percentage(disk)}")

    # SYMBIOTIC ALIGNMENT section
    lines.append("\n🔗 SYMBIOTIC ALIGNMENT")
    lines.append("-" * 70)
    lines.append(f"  Alignment:   {_render_progress_bar(state.alignment_score, 1.0, 15)} "
                 f"{_format_percentage(state.alignment_score * 100)}")

    if state.conflicts:
        lines.append(f"  Conflicts:")
        for conflict in state.conflicts:
            lines.append(f"    ⚠️  {conflict}")
    else:
        lines.append(f"  Conflicts:   (none)")

    if state.shared_objectives:
        lines.append(f"  Shared Objectives:")
        for obj in state.shared_objectives:
            lines.append(f"    ✓ {obj}")

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
    lines.append(" 💭 CYNIC MIND - Deep Dive")
    lines.append("=" * 70)

    # Observations
    lines.append("\n📋 OBSERVATIONS")
    lines.append("-" * 70)
    if state.cynic_observations:
        for key, value in state.cynic_observations.items():
            lines.append(f"  {key:.<40} {value}")
    else:
        lines.append("  (no observations)")

    # Current Thinking
    lines.append("\n🤔 CURRENT THINKING")
    lines.append("-" * 70)
    lines.append(f"  {state.cynic_thinking}")

    # Confidence
    lines.append(f"\n  Confidence Level:")
    lines.append(f"    {_render_progress_bar(state.cynic_confidence, 1.0, 30)} "
                 f"{_format_percentage(state.cynic_confidence * 100)}")

    # Planning
    lines.append("\n📋 PLANNING ITEMS")
    lines.append("-" * 70)
    if state.cynic_planning:
        for idx, item in enumerate(state.cynic_planning, 1):
            lines.append(f"  {idx}. {item}")
    else:
        lines.append("  (no planning items)")

    # E-Score
    lines.append("\n⚡ E-SCORE (Energy/Consciousness)")
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
    lines.append(" ⚙️  MACHINE - Resources & Health")
    lines.append("=" * 70)

    # Resource percentages section
    lines.append("\n📊 RESOURCE UTILIZATION")
    lines.append("-" * 70)

    cpu = state.machine_resources.get('cpu_percent', 0.0)
    memory = state.machine_resources.get('memory_percent', 0.0)
    disk = state.machine_resources.get('disk_percent', 0.0)
    temp = state.machine_resources.get('temperature', 0.0)
    network = state.machine_resources.get('network_bandwidth', 0.0)

    lines.append(f"  CPU:")
    lines.append(f"    {_render_progress_bar(cpu, 100.0, 30)} {_format_percentage(cpu)}")

    lines.append(f"  Memory:")
    lines.append(f"    {_render_progress_bar(memory, 100.0, 30)} {_format_percentage(memory)}")

    lines.append(f"  Disk:")
    lines.append(f"    {_render_progress_bar(disk, 100.0, 30)} {_format_percentage(disk)}")

    if temp > 0:
        lines.append(f"  Temperature:")
        lines.append(f"    {temp:.1f}°C")

    if network > 0:
        lines.append(f"  Network Bandwidth:")
        lines.append(f"    {network:.0f} bytes/sec")

    # Health indicators
    lines.append("\n✅ HEALTH INDICATORS")
    lines.append("-" * 70)

    is_healthy = state.machine_health.get('is_healthy', False)
    health_icon = "✓" if is_healthy else "✗"
    lines.append(f"  Overall Health:    {health_icon}")

    cpu_ok = state.machine_health.get('cpu_ok', False)
    cpu_icon = "✓" if cpu_ok else "✗"
    lines.append(f"  CPU OK:            {cpu_icon}")

    memory_ok = state.machine_health.get('memory_ok', False)
    memory_icon = "✓" if memory_ok else "✗"
    lines.append(f"  Memory OK:         {memory_icon}")

    disk_ok = state.machine_health.get('disk_ok', False)
    disk_icon = "✓" if disk_ok else "✗"
    lines.append(f"  Disk OK:           {disk_icon}")

    # Constraints and warnings
    constraints = state.machine_constraints.get('warnings', [])
    if constraints:
        lines.append("\n⚠️  CONSTRAINTS & WARNINGS")
        lines.append("-" * 70)
        for constraint in constraints:
            lines.append(f"  • {constraint}")
    else:
        lines.append("\n⚠️  CONSTRAINTS & WARNINGS")
        lines.append("-" * 70)
        lines.append("  (no warnings)")

    # Capability delta
    if state.machine_capability_delta:
        lines.append("\n🔄 CAPABILITY CHANGES")
        lines.append("-" * 70)
        for delta in state.machine_capability_delta:
            lines.append(f"  • {delta}")

    lines.append("\n" + "=" * 70)

    return "\n".join(lines)
