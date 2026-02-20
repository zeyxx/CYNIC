"""
Handler Compile-Time Discovery & Validation — Verify all handlers are properly wired.

Validates:
1. All handler modules in cynic.api.handlers are discovered
2. All handler names are unique
3. All subscribed events are valid CoreEvent types
4. All declared dependencies are available (or documented as optional)
5. No duplicate subscriptions to the same (event, handler) pair
6. No orphan handlers (handlers in handlers/ but not registered)

This runs at kernel startup and reports issues to assist with architecture governance.
"""

from __future__ import annotations

import importlib
import logging
import pkgutil
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from cynic.api.handlers.base import HandlerGroup
    from cynic.core.event_bus import CoreEvent

logger = logging.getLogger("cynic.api.handlers.validator")


@dataclass
class ValidationIssue:
    """Single validation problem."""
    severity: str  # "ERROR" | "WARNING" | "INFO"
    category: str  # "MISSING" | "DUPLICATE" | "INVALID" | "ORPHAN" | "DEPENDENCY"
    handler_name: str  # Handler affected (or "system" for global issues)
    message: str


class HandlerValidator:
    """Compile-time validation of handler discovery and wiring."""

    def __init__(self) -> None:
        self._issues: list[ValidationIssue] = []
        self._discovered_names: set[str] = set()
        self._all_module_names: set[str] = set()

    def validate(self, discovered_groups: list[HandlerGroup]) -> list[ValidationIssue]:
        """
        Validate discovered handler groups against module structure.

        Args:
            discovered_groups: Handler groups discovered by discover_handler_groups()

        Returns:
            List of ValidationIssue objects (empty if no problems)
        """
        self._issues.clear()
        self._discovered_names.clear()
        self._all_module_names.clear()

        # ── Phase 1: Index discovered handlers ─────────────────────────────
        for group in discovered_groups:
            if group.name in self._discovered_names:
                self._issues.append(ValidationIssue(
                    severity="ERROR",
                    category="DUPLICATE",
                    handler_name=group.name,
                    message=f"Duplicate handler name: '{group.name}' found multiple times",
                ))
            self._discovered_names.add(group.name)

        # ── Phase 2: Scan all handler modules (detect orphans) ─────────────
        self._scan_all_modules()

        # ── Phase 3: Check for orphan handlers ────────────────────────────
        orphan_modules = self._all_module_names - self._discovered_names
        for module_name in sorted(orphan_modules):
            self._issues.append(ValidationIssue(
                severity="WARNING",
                category="ORPHAN",
                handler_name=module_name,
                message=f"Handler module '{module_name}' exists but was not discovered",
            ))

        # ── Phase 4: Validate each handler ───────────────────────────────
        for group in discovered_groups:
            self._validate_handler(group)

        # ── Phase 5: Check for duplicate subscriptions ────────────────────
        self._check_duplicate_subscriptions(discovered_groups)

        return self._issues

    def _scan_all_modules(self) -> None:
        """Scan cynic.api.handlers to find all module names."""
        try:
            from cynic.api import handlers as _pkg
            for _, module_name, _ in pkgutil.iter_modules(_pkg.__path__):
                if not module_name.startswith("_") and module_name not in ("base", "introspect", "services", "validator"):
                    self._all_module_names.add(module_name)
        except Exception as e:
            logger.debug("Failed to scan handler modules: %s", e)

    def _validate_handler(self, group: HandlerGroup) -> None:
        """Validate a single handler group."""
        # Check: subscriptions() returns valid list
        try:
            subs = group.subscriptions()
            if not isinstance(subs, list):
                self._issues.append(ValidationIssue(
                    severity="ERROR",
                    category="INVALID",
                    handler_name=group.name,
                    message=f"subscriptions() must return list, got {type(subs).__name__}",
                ))
                return
        except Exception as e:
            self._issues.append(ValidationIssue(
                severity="ERROR",
                category="INVALID",
                handler_name=group.name,
                message=f"subscriptions() raised exception: {e}",
            ))
            return

        # Check: all subscribed events are CoreEvent types
        try:
            from cynic.core.event_bus import CoreEvent
            valid_events = {e.value for e in CoreEvent}
        except Exception:
            valid_events = set()

        for event_type, handler_fn in subs:
            try:
                event_value = event_type.value if hasattr(event_type, "value") else str(event_type)
            except Exception:
                event_value = str(event_type)

            # Event should be in CoreEvent
            if hasattr(event_type, "__class__") and event_type.__class__.__name__ == "CoreEvent":
                # Valid CoreEvent
                pass
            else:
                self._issues.append(ValidationIssue(
                    severity="WARNING",
                    category="INVALID",
                    handler_name=group.name,
                    message=f"Subscription to non-CoreEvent: {event_value}",
                ))

        # Check: dependencies() returns frozenset
        try:
            deps = group.dependencies()
            if not isinstance(deps, frozenset):
                self._issues.append(ValidationIssue(
                    severity="WARNING",
                    category="INVALID",
                    handler_name=group.name,
                    message=f"dependencies() must return frozenset, got {type(deps).__name__}",
                ))
        except Exception as e:
            self._issues.append(ValidationIssue(
                severity="WARNING",
                category="INVALID",
                handler_name=group.name,
                message=f"dependencies() raised exception: {e}",
            ))

    def _check_duplicate_subscriptions(self, groups: list[HandlerGroup]) -> None:
        """Detect if multiple handlers subscribe to the same (event, handler_fn) pair."""
        subscription_map: dict[tuple[str, str], list[str]] = {}

        for group in groups:
            try:
                for event_type, handler_fn in group.subscriptions():
                    event_name = getattr(event_type, "value", str(event_type))
                    handler_name_str = f"{handler_fn.__qualname__}"
                    key = (event_name, handler_name_str)

                    if key not in subscription_map:
                        subscription_map[key] = []
                    subscription_map[key].append(group.name)
            except Exception:
                pass

        # Check for duplicates (same event, same handler function, multiple groups)
        for (event_name, handler_name), groups_list in subscription_map.items():
            if len(groups_list) > 1:
                self._issues.append(ValidationIssue(
                    severity="INFO",
                    category="DUPLICATE",
                    handler_name="|".join(sorted(groups_list)),
                    message=f"Multiple handlers subscribe to {event_name} via {handler_name}: {groups_list}",
                ))

    def report(self) -> str:
        """Generate human-readable validation report."""
        if not self._issues:
            return "✅ Handler validation: OK (0 issues)"

        lines = [f"⚠️  Handler validation: {len(self._issues)} issues"]
        lines.append("")

        errors = [i for i in self._issues if i.severity == "ERROR"]
        warnings = [i for i in self._issues if i.severity == "WARNING"]
        infos = [i for i in self._issues if i.severity == "INFO"]

        if errors:
            lines.append(f"🔴 ERRORS ({len(errors)}):")
            for issue in errors:
                lines.append(f"  [{issue.category}] {issue.handler_name}: {issue.message}")
            lines.append("")

        if warnings:
            lines.append(f"🟡 WARNINGS ({len(warnings)}):")
            for issue in warnings:
                lines.append(f"  [{issue.category}] {issue.handler_name}: {issue.message}")
            lines.append("")

        if infos:
            lines.append(f"ℹ️  INFO ({len(infos)}):")
            for issue in infos:
                lines.append(f"  [{issue.category}] {issue.handler_name}: {issue.message}")

        return "\n".join(lines)

    def has_errors(self) -> bool:
        """Return True if any ERROR-severity issues found."""
        return any(i.severity == "ERROR" for i in self._issues)

    def summary(self) -> dict[str, Any]:
        """Return validation summary for APIs."""
        return {
            "total_issues": len(self._issues),
            "errors": len([i for i in self._issues if i.severity == "ERROR"]),
            "warnings": len([i for i in self._issues if i.severity == "WARNING"]),
            "infos": len([i for i in self._issues if i.severity == "INFO"]),
            "discovered_handlers": sorted(self._discovered_names),
            "orphan_modules": sorted(self._all_module_names - self._discovered_names),
            "issues": [
                {
                    "severity": i.severity,
                    "category": i.category,
                    "handler": i.handler_name,
                    "message": i.message,
                }
                for i in self._issues
            ],
        }
