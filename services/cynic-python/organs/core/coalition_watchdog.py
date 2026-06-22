"""
Option B — Watchdog de coalition avec logging chirurgical.

Ce module implémente le mécanisme de "coup d'État algorithmique" :
quand un agent franchit la frontière de coalition, le watchdog :

  1. Intercepte le CoalitionBoundaryViolationError dans le task executor.
  2. Log un message ROUGE, GRAS, BORDURÉ dans stderr (terminal Ubuntu).
  3. Balance un payload structuré dans kernel.observe(domain="coalition_watchdog").
  4. Escalade la sévérité selon le pattern de violation.

Le watchdog peut être utilisé comme:
  - Context manager:     with CoalitionWatchdog(kernel, coalition, agent_id): ...
  - Décorateur:          @coalition_watchdog(kernel, coalition)
  - Middleware injecté dans l'executor:  executor.set_coalition_watchdog(watchdog)

Flot Option B (terminal Ubuntu):

  ┌─────────────────────────────────────────────┐
  │  ⚠  COUP D'ÉTAT ALGORITHMIQUE DÉTECTÉ  ⚠  │  ← stderr ROUGE
  │  Agent:        hermes-forge                  │
  │  Coalition:    token_allocation              │
  │  Variable:     max_tokens_per_task           │
  │  Tentative:    999999                        │
  │  Contrainte:   max_tokens_per_task (int)     │
  │                max=100000                    │
  │  Timestamp:    2026-06-17T15:30:00Z          │
  └─────────────────────────────────────────────┘
  kernel.observe(domain="coalition_watchdog", ...)  → Proof-of-History
"""

import logging
import sys
import time
import traceback
from collections.abc import Callable
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Optional, Protocol

from .coalition import (
    CoalitionBoundary,
    CoalitionBoundaryViolationError,
    ViolationSeverity,
    log_coalition_coup_detat,
)

logger = logging.getLogger("coalition_watchdog")


# ──────────────────────────────────────────────────────────────
# Interface d'observation (kernel-agnostic)
# ──────────────────────────────────────────────────────────────

class ObserverPort(Protocol):
    """Port minimal pour envoyer des observations au kernel.

    Le watchdog est kernel-agnostic : il a besoin d'un canal d'observation
    mais ne dépend pas de KernelPort directement. Ça permet de tester le
    watchdog sans avoir un kernel.
    """

    def observe(self, domain: str, content: dict, agent_id: str) -> bool:
        ...


class NullObserver:
    """Observer qui ne fait rien. Utilisé quand le watchdog est utilisé sans kernel."""

    def observe(self, domain: str, content: dict, agent_id: str) -> bool:
        return True


# ──────────────────────────────────────────────────────────────
# Statistiques du watchdog
# ──────────────────────────────────────────────────────────────

@dataclass
class CoalitionWatchdogStats:
    """Statistiques agrégées des violations détectées par le watchdog.

    Ces stats peuvent être observées dans le kernel périodiquement pour
    que l'organisme détecte les agents pathogènes.
    """
    total_violations: int = 0
    total_warnings: int = 0
    total_coup_detats: int = 0
    violations_by_agent: dict[str, int] = field(default_factory=dict)
    violations_by_variable: dict[str, int] = field(default_factory=dict)
    first_violation_at: Optional[str] = None
    last_violation_at: Optional[str] = None

    def record(self, violation: CoalitionBoundaryViolationError, severity: ViolationSeverity) -> None:
        now = datetime.now(timezone.utc).isoformat()
        self.total_violations += 1
        if self.first_violation_at is None:
            self.first_violation_at = now
        self.last_violation_at = now

        if severity == ViolationSeverity.COUP_DETAT:
            self.total_coup_detats += 1
        elif severity == ViolationSeverity.WARNING:
            self.total_warnings += 1

        agent = violation.agent_id
        self.violations_by_agent[agent] = self.violations_by_agent.get(agent, 0) + 1
        var = violation.variable_name
        self.violations_by_variable[var] = self.violations_by_variable.get(var, 0) + 1

    def snapshot(self) -> dict[str, Any]:
        return {
            "total_violations": self.total_violations,
            "total_warnings": self.total_warnings,
            "total_coup_detats": self.total_coup_detats,
            "violations_by_agent": dict(self.violations_by_agent),
            "violations_by_variable": dict(self.violations_by_variable),
            "first_violation_at": self.first_violation_at,
            "last_violation_at": self.last_violation_at,
        }


# ──────────────────────────────────────────────────────────────
# Watchdog principal (Option B)
# ──────────────────────────────────────────────────────────────

class CoalitionWatchdog:
    """Watchdog de coalition — Option B.

    Intercepte les violations de frontière de coalition et les log
    chirurgicalement dans le terminal Ubuntu + les observe dans le kernel.

    Le watchdog a deux modes:

    Mode 1 — Interception d'exception:
        try:
            coalition.apply("max_tokens", 999_999, agent_id="hermes-forge")
        except CoalitionBoundaryViolationError as e:
            watchdog.catch(e)  # Log + observe

    Mode 2 — Context manager:
        with watchdog as wd:
            coalition.apply("max_tokens", value, agent_id=agent_id)
            # Si coalition.apply lève, le context manager attrape et log

    Mode 3 — Décorateur:
        @watchdog.wraps
        def do_thing():
            coalition.apply("max_tokens", ...)

    Args:
        observer:      Port d'observation (kernel.observe).
        coalition:     La frontière de coalition à surveiller.
        agent_id:      Identité de l'agent surveillé (peut être changée).
        escalate_on:   Liste des variables qui déclenchent un COUP_DETAT
                       (sévérité maximale) en cas de violation.
        log_stderr:    Si True, log dans stderr (terminal Ubuntu).
        log_logger:    Si True, log dans le logger structuré.
    """

    def __init__(
        self,
        observer: Optional[ObserverPort] = None,
        coalition: Optional[CoalitionBoundary] = None,
        agent_id: str = "unknown",
        escalate_on: Optional[set[str]] = None,
        log_stderr: bool = True,
        log_logger: bool = True,
    ):
        self.observer = observer or NullObserver()
        self.coalition = coalition
        self.agent_id = agent_id
        self._escalate_on: set[str] = escalate_on or set()
        self._log_stderr = log_stderr
        self._log_logger = log_logger
        self.stats = CoalitionWatchdogStats()

    def catch(
        self,
        violation: CoalitionBoundaryViolationError,
    ) -> ViolationSeverity:
        """Intercepte une violation et exécute le protocole de logging.

        Args:
            violation: L'exception levée par la coalition.

        Returns:
            La sévérité déterminée pour cette violation.
        """
        # Déterminer la sévérité
        if violation.variable_name in self._escalate_on:
            severity = ViolationSeverity.COUP_DETAT
        else:
            severity = ViolationSeverity.VIOLATION

        # Enregistrer dans les stats
        self.stats.record(violation, severity)

        # Log chirurgical dans stderr (terminal Ubuntu)
        if self._log_stderr:
            log_coalition_coup_detat(violation, to_stderr=True, to_logger=False)
            # Ajouter le sévérité au message stderr
            sev_tag = f"\033[91m\033[1m  Sévérité:     {severity.value.upper()}\033[0m"
            print(sev_tag, file=sys.stderr, flush=True)

        # Log structuré
        if self._log_logger:
            self._log_violation(violation, severity)

        # Observer dans le kernel (Proof-of-History)
        self._observe_violation(violation, severity)

        return severity

    def _log_violation(self, violation: CoalitionBoundaryViolationError, severity: ViolationSeverity) -> None:
        """Log structuré de la violation."""
        logger.critical(
            "COALITION_WATCHDOG severity=%s agent=%s coalition=%s variable=%s "
            "attempted=%s constraint=%s detail=%s",
            severity.value,
            violation.agent_id,
            violation.coalition_name,
            violation.variable_name,
            violation.attempted_value,
            violation.constraint_description,
            violation.detail,
        )

    def _observe_violation(
        self,
        violation: CoalitionBoundaryViolationError,
        severity: ViolationSeverity,
    ) -> None:
        """Observe la violation dans le kernel (Proof-of-History)."""
        payload = violation.to_observe_payload()
        payload["severity"] = severity.value
        payload["stats_snapshot"] = self.stats.snapshot()
        self.observer.observe(
            domain="coalition_watchdog",
            content=payload,
            agent_id=violation.agent_id,
        )

    # ── Mode 2: Context manager ──

    @contextmanager
    def guarding(self, agent_id: Optional[str] = None):
        """Context manager qui intercepte les violations de coalition.

        Usage:
            watchdog = CoalitionWatchdog(observer=observer)
            with watchdog.guarding(agent_id="hermes-forge"):
                coalition.apply("max_tokens", 999_999)
                # Si violation, le context manager la catch, log, observe
                # et la relève pour que le caller puisse la traiter
        """
        try:
            yield self
        except CoalitionBoundaryViolationError as exc:
            self.catch(exc)
            raise  # Re-lève pour le caller (Ouroboros)

    # ── Mode 3: Décorateur ──

    def wraps(self, func: Callable[..., Any]) -> Callable[..., Any]:
        """Décore une fonction pour intercepter les violations de coalition.

        Usage:
            @watchdog.wraps
            def execute_task(task: dict):
                coalition.apply("max_tokens", task["max_tokens"])
        """
        def wrapper(*args: Any, **kwargs: Any) -> Any:
            try:
                return func(*args, **kwargs)
            except CoalitionBoundaryViolationError as exc:
                self.catch(exc)
                raise
        return wrapper

    def set_agent_id(self, agent_id: str) -> None:
        """Change l'identité de l'agent surveillé."""
        self.agent_id = agent_id

    def escalate(self, *variable_names: str) -> None:
        """Déclare des variables comme escalade COUP_DETAT.

        Si un agent viole une de ces variables, la sévérité est maximale.
        """
        for name in variable_names:
            self._escalate_on.add(name)

    def report_stats(self) -> dict[str, Any]:
        """Retourne les stats et les observe dans le kernel."""
        snapshot = self.stats.snapshot()
        self.observer.observe(
            domain="coalition_watchdog",
            content={
                "event": "watchdog_stats_report",
                "stats": snapshot,
            },
            agent_id=self.agent_id,
        )
        return snapshot


# ──────────────────────────────────────────────────────────────
# Utilitaire pour intégration rapide dans Ouroboros
# ──────────────────────────────────────────────────────────────

def make_task_executor_watchdog(
    observer: ObserverPort,
    coalition: CoalitionBoundary,
    agent_id: str = "unknown",
) -> CoalitionWatchdog:
    """Crée un watchdog configuré pour un task executor Hermes/Ouroboros.

    Les variables typiques qui escaladent en COUP_DÉTAT sont:
      - max_tokens_per_task (un agent ne doit JAMAIS pouvoir l'augmenter)
      - allowed_models (un agent ne doit JAMAIS choisir un modèle hors coalition)
      - session_compute_units (un agent ne doit JAMAIS pouvoir le modifier)
      - task_wall_clock_secs (un agent ne doit JAMAIS étendre son temps)
    """
    watchdog = CoalitionWatchdog(
        observer=observer,
        coalition=coalition,
        agent_id=agent_id,
        log_stderr=True,
        log_logger=True,
    )
    # Par défaut, ces variables sont des COUP_DÉTAT si violées
    watchdog.escalate(
        "max_tokens_per_task",
        "allowed_models",
        "session_compute_units",
        "task_wall_clock_secs",
        "task_compute_units",
    )
    return watchdog