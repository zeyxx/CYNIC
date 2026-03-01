"""
Audit factory.py to ensure all components are initialized and passed to Organism.

Checks:
  - All components initialized (bus, journal, tracer, collector, etc.)
  - All components injected into ArchiveCore
  - All service setters called (set_qtable, set_metrics_collector, etc.)
  - No missing wiring steps
"""
import re
from pathlib import Path


def audit_factory():
    """Check factory.py for completeness."""
    factory_path = Path("cynic/kernel/organism/factory.py")
    content = factory_path.read_text(encoding='utf-8')

    issues = []

    # Required initializations (accept EventBus or get_core_bus pattern)
    required_inits = [
        ("EventBus initialization", lambda c: "instance_bus = get_core_bus" in c or "instance_bus = EventBus" in c),
        ("EventJournal", lambda c: "self.journal = EventJournal" in c),
        ("DecisionTracer", lambda c: "self.tracer = DecisionTracer" in c),
        ("LoopClosureValidator", lambda c: "self.loop_validator = LoopClosureValidator" in c),
        ("StateReconstructor", lambda c: "self.reconstructor = StateReconstructor" in c),
        ("EventMetricsCollector", lambda c: "self.metrics_collector = EventMetricsCollector" in c),
    ]

    for init_name, check in required_inits:
        if not check(content):
            issues.append(f"Missing initialization: {init_name}")

    # Required service injections (setters)
    required_setters = [
        "prober.set_qtable",
        "prober.set_residual_detector",
        "prober.set_escore_tracker",
    ]

    for setter in required_setters:
        if setter not in content:
            issues.append(f"Missing service injection: {setter}")

    # Required ArchiveCore fields
    required_archive_fields = [
        "journal=self.journal",
        "loop_validator=self.loop_validator",
        "reconstructor=self.reconstructor",
        "metrics_collector=self.metrics_collector",
    ]

    for field in required_archive_fields:
        if field not in content:
            issues.append(f"Missing ArchiveCore field: {field}")

    # Required bus handlers
    required_handlers = [
        'instance_bus.on("*", self._journal_adapter.on_event)',
        'instance_bus.on("*", self._loop_adapter.on_event)',
        'instance_bus.on("*", self._metrics_adapter.on_event)',
    ]

    for handler in required_handlers:
        # Normalize whitespace for matching
        handler_normalized = re.sub(r'\s+', ' ', handler)
        content_normalized = re.sub(r'\s+', ' ', content)
        if handler_normalized not in content_normalized:
            issues.append(f"Missing bus handler: {handler}")

    if issues:
        print(f"[FAIL] Factory wiring audit found {len(issues)} issues:")
        for issue in issues:
            print(f"  - {issue}")
        return False
    else:
        print("[PASS] Factory wiring is complete")
        return True


if __name__ == "__main__":
    import sys
    if not audit_factory():
        sys.exit(1)
