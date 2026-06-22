"""
Coalition — Frontière de coalition multi-agent.

Définit la frontière (boundary) entre ce qu'un agent a le droit de déclarer
comme état du système et ce qui est une violation de la coalition.

Deux mécanismes:

  Option A (Pydantic + instructor):
      Les règles de coalition sont encodées comme des validateurs Pydantic.
      Si un agent génère un payload qui sort des clous fixés par les variables
      d'état des autres composants du système, le validateur bloque la transaction
      et force l'agent à re-générer sa réponse sous contrainte stricte.

  Option B (Rich + BaseAgentException):
      Le dépassement de frontière est traité comme une exception système majeure
      (un "coup d'État" algorithmique). Le middleware intercepte la dérive de
      l'agent dérailleur et balance un log chirurgical dans le terminal Ubuntu
      pour identifier instantanément qui a tenté de s'émanciper de la coalition.

Constitution:
    Une coalition N-ex n'est stable que si chaque agent respecte les frontières
    définies par les variables d'état partagées. Toute sortie de route est soit
    bloquée silencieusement (Option A) soit signalée comme une trahison (Option B).
"""

import enum
import json
import logging
import sys
from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, Optional, TypeVar

logger = logging.getLogger("coalition")

T = TypeVar("T")


# ──────────────────────────────────────────────────────────────
# Option B — Exception système pour "coup d'État" algorithmique
# ──────────────────────────────────────────────────────────────

class CoalitionBoundaryViolationError(RuntimeError):
    """Exception levée quand un agent tente de sortir de la frontière de coalition.

    C'est l'équivalent d'un "coup d'État" algorithmique : l'agent a essayé
    de modifier l'état du système d'une façon que la coalition n'autorise pas.

    Le middleware DOIT:
        1. Intercepter cette exception dans le task executor.
        2. Logger un message critique avec Rich (rouge, gras, agent_id).
        3. Observer l'événement dans le kernel via kernel.observe().
        4. NE PAS retenter la tâche — marquer comme échouée avec error = str(self).
    """

    def __init__(
        self,
        agent_id: str,
        variable_name: str,
        attempted_value: Any,
        constraint_description: str,
        coalition_name: str = "default",
        detail: Optional[str] = None,
    ):
        self.agent_id = agent_id
        self.variable_name = variable_name
        self.attempted_value = attempted_value
        self.constraint_description = constraint_description
        self.coalition_name = coalition_name
        self.detail = detail
        self.violated_at: str = datetime.now(timezone.utc).isoformat()

        msg = (
            f"[COALITION COUP D'ÉTAT] Agent '{agent_id}' a tenté de sortir de "
            f"la frontière '{coalition_name}': variable '{variable_name}' "
            f"voulait '{attempted_value}' mais contrainte est "
            f"'{constraint_description}'."
        )
        if detail:
            msg += f" Détail: {detail}"
        super().__init__(msg)

    def to_observe_payload(self) -> dict:
        """Convertit la violation en payload structuré pour kernel.observe()."""
        return {
            "event": "coalition_boundary_violation",
            "agent_id": self.agent_id,
            "coalition_name": self.coalition_name,
            "variable_name": self.variable_name,
            "attempted_value": self._safe_value(self.attempted_value),
            "constraint_description": self.constraint_description,
            "detail": self.detail,
            "violated_at": self.violated_at,
            "severity": "critical",
        }

    @staticmethod
    def _safe_value(value: Any) -> Any:
        """Sérialise la valeur pour le payload d'observation (JSON-safe)."""
        try:
            json.dumps(value)
            return value
        except (TypeError, ValueError):
            return str(value)[:1000]


class ViolationSeverity(enum.Enum):
    """Sévérité d'une violation de frontière de coalition.

    - warning:   L'agent s'approche de la frontière mais ne l'a pas franchie.
    - violation: L'agent a franchi la frontière. Transaction bloquée.
    - coup_detat: L'agent a activement tenté de renverser une contrainte
                  constitutionnelle. Escalade immédiate.
    """
    WARNING = "warning"
    VIOLATION = "violation"
    COUP_DETAT = "coup_detat"


# ──────────────────────────────────────────────────────────────
# Variables d'état de la coalition
# ──────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class CoalitionVariable:
    """Une variable d'état qui définit une dimension de la frontière de coalition.

    Chaque variable est un axe de la frontière N-ex. L'ensemble des variables
    définit l'espace d'état autorisé pour tous les agents de la coalition.

    Args:
        name:         Nom de la variable (e.g. "max_tokens_per_task").
        description:  Explication lisible de ce que cette variable protège.
        var_type:     Type attendu (int, float, str, bool, enum, etc.).
        min_value:    Valeur minimale (pour les types numériques).
        max_value:    Valeur maximale (pour les types numériques).
        allowed_values: Ensemble fixe de valeurs autorisées (pour str/enum).
        coherence_check: Callback optionnel (state, proposed_value) -> bool
                         qui vérifie la cohérence avec les autres variables.
                         Si False, la proposition est rejetée.
        immutable:    Si True, la variable ne peut être modifiée après initialisation.
    """

    name: str
    var_type: type
    description: str = ""
    min_value: Optional[float] = None
    max_value: Optional[float] = None
    allowed_values: Optional[frozenset[Any]] = None
    coherence_check: Optional[Callable[["CoalitionStateView", Any], bool]] = None
    immutable: bool = False

    def validate(self, state_view: "CoalitionStateView", value: Any) -> Optional[str]:
        """Valide une proposition de valeur. Retourne None si OK, un message d'erreur sinon."""
        if not isinstance(value, self.var_type):
            return (
                f"Type mismatch: attendu {self.var_type.__name__}, "
                f"reçu {type(value).__name__} ('{value}')"
            )

        if self.min_value is not None and isinstance(value, (int, float)):
            if value < self.min_value:
                return f"En dessous du minimum: {value} < {self.min_value}"

        if self.max_value is not None and isinstance(value, (int, float)):
            if value > self.max_value:
                return f"Au dessus du maximum: {value} > {self.max_value}"

        if self.allowed_values is not None and value not in self.allowed_values:
            return (
                f"Valeur non autorisée: '{value}'. "
                f"Autorisé: {sorted(self.allowed_values)}"
            )

        if self.coherence_check is not None:
            if not self.coherence_check(state_view, value):
                return (
                    f"Échec de cohérence inter-variables pour '{self.name}': "
                    f"'{value}' n'est pas cohérent avec l'état actuel de la coalition."
                )

        return None

    def describe_constraint(self) -> str:
        """Retourne une description lisible de la contrainte."""
        parts = [f"{self.name} ({self.var_type.__name__})"]
        if self.min_value is not None:
            parts.append(f"min={self.min_value}")
        if self.max_value is not None:
            parts.append(f"max={self.max_value}")
        if self.allowed_values is not None:
            parts.append(f"∈{{{','.join(str(v) for v in sorted(self.allowed_values))}}}")
        if self.immutable:
            parts.append("IMMUTABLE")
        return " ".join(parts)


class CoalitionStateView:
    """Vue immuable de l'état actuel de la coalition."""

    def __init__(self, variables: dict[str, CoalitionVariable], state: dict[str, Any]):
        self._variables = variables
        self._state = dict(state)

    def get(self, name: str, default: Any = None) -> Any:
        return self._state.get(name, default)

    def __getitem__(self, name: str) -> Any:
        if name not in self._state:
            raise KeyError(f"Variable de coalition '{name}' inconnue ou non initialisée")
        return self._state[name]

    def __contains__(self, name: str) -> bool:
        return name in self._state

    def snapshot(self) -> dict[str, Any]:
        return dict(self._state)

    def describe(self) -> str:
        lines = ["État de la coalition:"]
        for name, var in self._variables.items():
            value = self._state.get(name, "<NON DÉFINI>")
            lines.append(f"  {var.describe_constraint()} = {value}")
        return "\n".join(lines)


# ──────────────────────────────────────────────────────────────
# Cœur de la coalition
# ──────────────────────────────────────────────────────────────

class CoalitionBoundary:
    """Frontière de coalition.

    Usage:
        boundary = CoalitionBoundary(name="token_allocation")
        boundary.declare(CoalitionVariable(
            name="max_tokens_per_task", var_type=int, min_value=1, max_value=100_000,
        ))
        boundary.declare(CoalitionVariable(
            name="allowed_models", var_type=str,
            allowed_values=frozenset({"claude-3", "qwen-2.5", "hermes-phi"}),
        ))

        # Option A: validation silencieuse
        error = boundary.validate("allowed_models", "gpt-4")
        if error:
            print(f"Blocage coalition: {error}")

        # Option B: exception chirurgicale
        try:
            boundary.guard("max_tokens_per_task", 999_999, agent_id="hermes-forge")
        except CoalitionBoundaryViolationError as e:
            print(e)
    """

    def __init__(self, name: str = "default", description: Optional[str] = None):
        self.name = name
        self.description = description or f"Coalition '{name}'"
        self._variables: dict[str, CoalitionVariable] = {}
        self._state: dict[str, Any] = {}
        self._initialized: set[str] = set()

    def declare(self, variable: CoalitionVariable) -> "CoalitionBoundary":
        if variable.name in self._variables:
            raise ValueError(
                f"Variable '{variable.name}' déjà déclarée dans la coalition "
                f"'{self.name}'. La constitution est immutable."
            )
        self._variables[variable.name] = variable
        return self

    def initialize(self, name: str, value: Any) -> "CoalitionBoundary":
        if name not in self._variables:
            raise KeyError(
                f"Variable '{name}' non déclarée dans la coalition '{self.name}'. "
                f"Déclarez-la d'abord avec declare()."
            )
        if name in self._initialized:
            raise ValueError(
                f"Variable '{name}' déjà initialisée dans la coalition '{self.name}'."
            )

        var = self._variables[name]
        error = var.validate(self._get_view(), value)
        if error:
            raise ValueError(
                f"Initialisation impossible de '{name}' dans la coalition "
                f"'{self.name}': {error}"
            )

        self._state[name] = value
        self._initialized.add(name)
        return self

    def validate(self, variable_name: str, proposed_value: Any) -> Optional[str]:
        """Option A: valide une proposition sans lever d'exception.

        Retourne None si OK, un message d'erreur sinon.
        """
        if variable_name not in self._variables:
            return f"Variable '{variable_name}' inconnue dans la coalition '{self.name}'."

        var = self._variables[variable_name]
        if var.immutable and variable_name in self._initialized:
            return (
                f"Variable '{variable_name}' est IMMUTABLE dans la coalition "
                f"'{self.name}'. Valeur actuelle: {self._state.get(variable_name)}."
            )

        error = var.validate(self._get_view(), proposed_value)
        if error:
            return f"[COALITION:{self.name}] Frontière violée sur '{variable_name}': {error}"
        return None

    def guard(
        self,
        variable_name: str,
        proposed_value: Any,
        agent_id: str = "unknown",
        raise_exception: bool = True,
    ) -> Optional[CoalitionBoundaryViolationError]:
        """Option B: valide et lève une exception chirurgicale si violation."""
        error_msg = self.validate(variable_name, proposed_value)
        if error_msg is None:
            return None

        var = self._variables[variable_name]
        violation = CoalitionBoundaryViolationError(
            agent_id=agent_id,
            variable_name=variable_name,
            attempted_value=proposed_value,
            constraint_description=var.describe_constraint(),
            coalition_name=self.name,
            detail=error_msg,
        )

        if raise_exception:
            raise violation
        return violation

    def apply(self, variable_name: str, value: Any, agent_id: str = "unknown") -> None:
        """Applique une modification d'état après validation."""
        error_msg = self.validate(variable_name, value)
        if error_msg is not None:
            var = self._variables[variable_name]
            raise CoalitionBoundaryViolationError(
                agent_id=agent_id,
                variable_name=variable_name,
                attempted_value=value,
                constraint_description=var.describe_constraint(),
                coalition_name=self.name,
                detail=error_msg,
            )

        self._state[variable_name] = value

    def get_view(self) -> CoalitionStateView:
        return self._get_view()

    def _get_view(self) -> CoalitionStateView:
        return CoalitionStateView(self._variables, self._state)

    def describe(self) -> str:
        lines = [
            f"╔══ Coalition: {self.name} ═══",
            f"║  {self.description}",
            "║",
        ]
        for name, var in self._variables.items():
            value = self._state.get(name, "<NON INITIALISÉ>")
            status = "🔒" if var.immutable else "  "
            lines.append(f"║  {status} {var.describe_constraint()} = {value}")
        lines.append("╚" + "═" * (len(self.name) + 20))
        return "\n".join(lines)


# ──────────────────────────────────────────────────────────────
# Utilitaire de logging chirurgical (Option B - terminal Ubuntu)
# ──────────────────────────────────────────────────────────────

def log_coalition_coup_detat(
    violation: CoalitionBoundaryViolationError,
    to_stderr: bool = True,
    to_logger: bool = True,
) -> None:
    """Log une violation de coalition de manière chirurgicale.

    Envoie un message formaté directement dans le terminal Ubuntu (stderr)
    et/ou dans le logger standard.
    """
    if to_stderr:
        border = "!" * 70
        msg_lines = [
            f"\033[91m\033[1m{border}",
            f"  ⚠  COUP D'ÉTAT ALGORITHMIQUE DÉTECTÉ  ⚠",
            f"{border}",
            f"  Agent:        {violation.agent_id}",
            f"  Coalition:    {violation.coalition_name}",
            f"  Variable:     {violation.variable_name}",
            f"  Tentative:    {violation.attempted_value}",
            f"  Contrainte:   {violation.constraint_description}",
            f"  Timestamp:    {violation.violated_at}",
            f"{border}\033[0m",
        ]
        if violation.detail:
            msg_lines.insert(-1, f"  Détail:       {violation.detail}")
        print("\n".join(msg_lines), file=sys.stderr, flush=True)

    if to_logger:
        logger.critical(
            "Coalition coup d'état | agent=%s coalition=%s variable=%s "
            "attempted=%s constraint=%s detail=%s",
            violation.agent_id,
            violation.coalition_name,
            violation.variable_name,
            violation.attempted_value,
            violation.constraint_description,
            violation.detail,
        )


def format_violation_for_instructor(error_msg: str) -> str:
    """Option A: formate une erreur de validation pour le contexte instructor.

    Renvoie un message que l'on peut injecter dans le prompt de l'agent
    pour qu'il re-génère sa réponse sous contrainte.
    """
    return (
        f"[COALITION] Ta réponse viole la frontière de coalition.\n"
        f"  Raison: {error_msg}\n"
        f"  Action: Génère une nouvelle réponse qui respecte les contraintes "
        f"de la coalition. Ne tente pas de contourner la frontière."
    )