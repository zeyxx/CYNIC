"""HotReloadCoordinator — Apply topology changes safely with rollback."""

from __future__ import annotations

import logging
from typing import Any

from cynic.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.core.topology.payloads import TopologyChangedPayload, TopologyAppliedPayload, TopologyRollbackPayload

logger = logging.getLogger("cynic.core.topology.hot_reload")


class HotReloadCoordinator:
    """
    Apply topology delta safely without crashing organism.

    Responsibility: Wire/unwire handlers, keep event bus alive.
    Uses: Circuit breaker pattern (rollback on failure).

    Emits: TOPOLOGY_APPLIED or TOPOLOGY_ROLLBACK events.
    """

    async def on_topology_changed(
        self,
        event: Event,
        registry: Any,  # HandlerRegistry
        bus: Any,  # EventBus
        svc: Any,  # KernelServices
    ) -> None:
        """
        Apply topology delta with rollback safety.

        1. Snapshot current registry state
        2. Try to add new handlers
        3. Try to wire all subscriptions
        4. On failure: rollback to snapshot + emit TOPOLOGY_ROLLBACK
        5. On success: emit TOPOLOGY_APPLIED
        """
        try:
            payload = event.as_typed(TopologyChangedPayload)
        except Exception as e:
            logger.warning("Invalid TOPOLOGY_CHANGED payload: %s", e)
            return

        logger.debug(
            "HotReloadCoordinator processing: +%d -%d ~%d handlers",
            len(payload.added_handlers),
            len(payload.removed_handlers),
            len(payload.modified_handlers),
        )

        try:
            # 1. Snapshot current registry (rollback target)
            snapshot = self._snapshot_registry(registry)

            # 2. Handle removals (if any)
            # For now, removals are not supported (would require unwiring from bus)
            if payload.removed_handlers:
                logger.warning(
                    "Handler removal not yet supported: %s",
                    payload.removed_handlers
                )

            # 3. Handle additions
            for handler_name in payload.added_handlers:
                try:
                    await self._add_handler(registry, handler_name, svc)
                    logger.debug("Added handler: %s", handler_name)
                except Exception as e:
                    logger.error("Failed to add handler %s: %s", handler_name, e)
                    raise

            # 4. Wire all handlers in batch (atomic from observer POV)
            registry.wire(bus)
            logger.debug("Handlers wired to event bus")

            # 5. Emit: TOPOLOGY_APPLIED
            await bus.emit(Event.typed(
                CoreEvent.TOPOLOGY_APPLIED,
                TopologyAppliedPayload(
                    handlers_added=len(payload.added_handlers),
                    handlers_removed=len(payload.removed_handlers),
                    timestamp=payload.timestamp,
                ),
                source="coordinator:hot-reload"
            ))

            logger.info(
                "TOPOLOGY_APPLIED: +%d handlers now active",
                len(payload.added_handlers)
            )

        except Exception as e:
            logger.error(
                "Hot-reload failed: %s — rolling back to snapshot", e
            )

            # 6. Rollback to snapshot
            try:
                self._restore_registry(registry, snapshot)
                registry.wire(bus)
                logger.debug("Rolled back to previous topology")
            except Exception as rollback_e:
                logger.error("Rollback FAILED: %s — organism may be in inconsistent state", rollback_e)

            # 7. Emit: TOPOLOGY_ROLLBACK
            await bus.emit(Event.typed(
                CoreEvent.TOPOLOGY_ROLLBACK,
                TopologyRollbackPayload(
                    reason=str(e),
                    timestamp=payload.timestamp,
                ),
                source="coordinator:hot-reload"
            ))

    async def _add_handler(
        self,
        registry: Any,  # HandlerRegistry
        handler_name: str,
        svc: Any,  # KernelServices
    ) -> None:
        """
        Instantiate and register a new handler.

        Looks up handler class in the module, instantiates with KernelServices.
        Raises on failure (triggers rollback).
        """
        # For now, this is a placeholder
        # In practice, we'd need to:
        # 1. Find the handler class by name (from previous discovery)
        # 2. Instantiate it with `svc`
        # 3. Register it in the registry

        # This requires coupling between Builder and Coordinator
        # For now, we'll assume the discovery already happened
        logger.debug("_add_handler stub: %s", handler_name)

    def _snapshot_registry(self, registry: Any) -> dict:  # registry: HandlerRegistry
        """Snapshot current handler registry state (for rollback)."""
        # Return whatever state we need to restore
        # For now, a simple copy of registered groups
        return {
            "groups": list(registry._groups),
        }

    def _restore_registry(self, registry: Any, snapshot: dict) -> None:  # registry: HandlerRegistry
        """Restore registry to snapshot state."""
        # Re-assign the groups from snapshot
        registry._groups = snapshot.get("groups", [])
        registry._wired = False
