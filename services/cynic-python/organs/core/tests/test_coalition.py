"""
Tests de la frontière de coalition multi-agent.

Couverture:
  - Option A: Pydantic + validate() silencieuse
  - Option B: CoalitionBoundaryViolationError + CoalitionWatchdog chirurgical
  - Ouroboros integration: coalition validation dans le task lifecycle
"""

import json
import sys
from io import StringIO
from typing import Any, Optional

import pytest

from ..coalition import (
    CoalitionBoundary,
    CoalitionBoundaryViolationError,
    CoalitionStateView,
    CoalitionVariable,
    ViolationSeverity,
    format_violation_for_instructor,
    log_coalition_coup_detat,
)
from ..coalition_watchdog import (
    CoalitionWatchdog,
    CoalitionWatchdogStats,
    NullObserver,
    ObserverPort,
    make_task_executor_watchdog,
)
from ..coalition_pydantic import CoalitionBaseModel, make_coalition_validator, coalition_field


# ════════════════════════════════════════════════════════════════
# Fixtures
# ════════════════════════════════════════════════════════════════

@pytest.fixture
def simple_coalition() -> CoalitionBoundary:
    """Coalition minimale pour les tests unitaires."""
    c = CoalitionBoundary(name="test_coalition", description="Coalition de test")
    c.declare(CoalitionVariable(
        name="max_tokens", var_type=int, min_value=1, max_value=100_000,
        description="Max tokens per task",
    ))
    c.declare(CoalitionVariable(
        name="model", var_type=str,
        allowed_values=frozenset({"claude-3", "qwen-2.5"}),
        description="Allowed model",
    ))
    c.declare(CoalitionVariable(
        name="enabled", var_type=bool,
        description="Feature flag",
    ))
    c.declare(CoalitionVariable(
        name="ratio", var_type=float, min_value=0.0, max_value=1.0,
        description="Ratio for some computation",
    ))
    c.declare(CoalitionVariable(
        name="name", var_type=str,
        description="Agent name",
    ))
    c.initialize("max_tokens", 50_000)
    c.initialize("model", "qwen-2.5")
    c.initialize("enabled", True)
    c.initialize("ratio", 0.5)
    c.initialize("name", "default-agent")
    return c


@pytest.fixture
def immortal_coalition() -> CoalitionBoundary:
    """Coalition avec variables immutables."""
    c = CoalitionBoundary(name="immortal_coalition")
    c.declare(CoalitionVariable(
        name="max_connections", var_type=int, min_value=1, max_value=10,
        immutable=True,
        description="Ne peut pas être changé après init",
    ))
    c.declare(CoalitionVariable(
        name="rate_limit", var_type=float, min_value=0.1, max_value=100.0,
        immutable=True,
        description="Rate limit immutable",
    ))
    c.initialize("max_connections", 5)
    c.initialize("rate_limit", 10.0)
    return c


class CaptureObserver(ObserverPort):
    """Observer qui capture les observations pour les tests."""
    def __init__(self):
        self.observations: list[dict] = []

    def observe(self, domain: str, content: dict, agent_id: str) -> bool:
        self.observations.append({
            "domain": domain,
            "content": content,
            "agent_id": agent_id,
        })
        return True


@pytest.fixture
def capture_observer() -> CaptureObserver:
    return CaptureObserver()


@pytest.fixture
def watchdog(simple_coalition, capture_observer) -> CoalitionWatchdog:
    w = CoalitionWatchdog(
        observer=capture_observer,
        coalition=simple_coalition,
        agent_id="test-agent",
        log_stderr=False,  # Silencieux dans les tests
        log_logger=False,
    )
    return w


# ════════════════════════════════════════════════════════════════
# Option A — Tests de validation silencieuse (CoalitionBoundary.validate)
# ════════════════════════════════════════════════════════════════

class TestOptionA_Validate:
    """Option A: validation silencieuse via CoaliationBoundary.validate()."""

    def test_valid_int(self, simple_coalition):
        assert simple_coalition.validate("max_tokens", 42_000) is None
        assert simple_coalition.validate("max_tokens", 1) is None
        assert simple_coalition.validate("max_tokens", 100_000) is None

    def test_invalid_int_below_min(self, simple_coalition):
        error = simple_coalition.validate("max_tokens", 0)
        assert error is not None
        assert "En dessous du minimum" in error

    def test_invalid_int_above_max(self, simple_coalition):
        error = simple_coalition.validate("max_tokens", 999_999)
        assert error is not None
        assert "Au dessus du maximum" in error

    def test_wrong_type(self, simple_coalition):
        error = simple_coalition.validate("max_tokens", "not_an_int")
        assert error is not None
        assert "Type mismatch" in error
        assert "str" in error
        assert "int" in error

    def test_valid_string_allowed(self, simple_coalition):
        assert simple_coalition.validate("model", "claude-3") is None
        assert simple_coalition.validate("model", "qwen-2.5") is None

    def test_invalid_string_not_allowed(self, simple_coalition):
        error = simple_coalition.validate("model", "gpt-4")
        assert error is not None
        assert "Valeur non autorisée" in error
        assert "gpt-4" in error
        assert "claude-3" in error

    def test_valid_bool(self, simple_coalition):
        assert simple_coalition.validate("enabled", True) is None
        assert simple_coalition.validate("enabled", False) is None

    def test_valid_float_range(self, simple_coalition):
        assert simple_coalition.validate("ratio", 0.0) is None
        assert simple_coalition.validate("ratio", 1.0) is None
        assert simple_coalition.validate("ratio", 0.75) is None

    def test_invalid_float_above_max(self, simple_coalition):
        error = simple_coalition.validate("ratio", 1.5)
        assert error is not None
        assert "Au dessus du maximum" in error

    def test_unknown_variable(self, simple_coalition):
        error = simple_coalition.validate("nonexistent", 42)
        assert error is not None
        assert "inconnue" in error
        assert "nonexistent" in error

    def test_immutable_rejected(self, immortal_coalition):
        error = immortal_coalition.validate("max_connections", 10)
        assert error is not None
        assert "IMMUTABLE" in error

    def test_instructor_format(self):
        """Le format instructor est directement utilisable dans un prompt LLM."""
        msg = format_violation_for_instructor("Test error message")
        assert "[COALITION]" in msg
        assert "Génère une nouvelle réponse" in msg
        assert "Test error message" in msg
        assert "contourner" in msg


# ════════════════════════════════════════════════════════════════
# Option A — Pydantic + instructor integration
# ════════════════════════════════════════════════════════════════

class TestOptionA_Pydantic:
    """Option A: intégration Pydantic avec CoalitionBaseModel."""

    def test_pydantic_valid_payload(self, simple_coalition):
        """Un payload valide passe sans erreur."""
        class TokenConfig(CoalitionBaseModel):
            _coalition = simple_coalition
            max_tokens: int
            model: str
            enabled: bool

        config = TokenConfig(max_tokens=42_000, model="qwen-2.5", enabled=True)
        assert config.max_tokens == 42_000
        assert config.model == "qwen-2.5"

    def test_pydantic_invalid_payload(self, simple_coalition):
        """Un payload invalide lève ValueError."""
        class TokenConfig(CoalitionBaseModel):
            _coalition = simple_coalition
            max_tokens: int
            model: str

        with pytest.raises(ValueError) as exc:
            TokenConfig(max_tokens=999_999, model="gpt-4")

        error_msg = str(exc.value)
        assert "[COALITION]" in error_msg
        assert "Au dessus du maximum" in error_msg
        assert "Valeur non autorisée" in error_msg

    def test_pydantic_no_coalition(self):
        """Sans coalition, CoalitionBaseModel se comporte comme BaseModel normal."""
        class FreeModel(CoalitionBaseModel):
            name: str

        m = FreeModel(name="anything")
        assert m.name == "anything"

    def test_instructor_validator_valid(self, simple_coalition):
        """Le validateur instructor accepte un modèle valide."""
        from pydantic import BaseModel
        validator = make_coalition_validator(simple_coalition)

        class TradeConfig(BaseModel):
            max_tokens: int
            model: str

        config = TradeConfig(max_tokens=10_000, model="claude-3")
        result = validator(config)
        assert result is config  # Retourné inchangé

    def test_instructor_validator_invalid(self, simple_coalition):
        """Le validateur instructor lève ValueError pour un modèle invalide."""
        from pydantic import BaseModel
        validator = make_coalition_validator(simple_coalition)

        class TradeConfig(BaseModel):
            max_tokens: int
            model: str

        config = TradeConfig(max_tokens=999_999, model="gpt-4")
        with pytest.raises(ValueError) as exc:
            validator(config)
        assert "[COALITION]" in str(exc.value)


# ════════════════════════════════════════════════════════════════
# Option B — CoalitionBoundaryViolationError (coup d'État)
# ════════════════════════════════════════════════════════════════

class TestOptionB_ViolationError:
    """Option B: CoalitionBoundaryViolationError — "coup d'État"."""

    def test_exception_creation(self):
        exc = CoalitionBoundaryViolationError(
            agent_id="rogue-agent",
            variable_name="max_tokens",
            attempted_value=999_999,
            constraint_description="max_tokens (int) max=100000",
            coalition_name="token_allocation",
            detail="Agent a tenté d'allouer trop de tokens",
        )
        assert exc.agent_id == "rogue-agent"
        assert exc.variable_name == "max_tokens"
        assert "[COALITION COUP D'ÉTAT]" in str(exc)
        assert "rogue-agent" in str(exc)
        assert "max_tokens" in str(exc)
        assert exc.violated_at is not None

    def test_observe_payload(self):
        exc = CoalitionBoundaryViolationError(
            agent_id="rogue-agent",
            variable_name="max_tokens",
            attempted_value=999_999,
            constraint_description="max_tokens (int) max=100000",
            coalition_name="token_allocation",
        )
        payload = exc.to_observe_payload()
        assert payload["event"] == "coalition_boundary_violation"
        assert payload["agent_id"] == "rogue-agent"
        assert payload["variable_name"] == "max_tokens"
        assert payload["severity"] == "critical"
        # JSON-safe
        assert json.dumps(payload)  # ne doit pas lever

    def test_guard_raises(self, simple_coalition):
        """CoalitionBoundary.guard() lève l'exception."""
        with pytest.raises(CoalitionBoundaryViolationError) as exc:
            simple_coalition.guard(
                "max_tokens", 999_999,
                agent_id="rogue-agent",
            )
        assert "999999" in str(exc.value)
        assert "rogue-agent" in str(exc.value)

    def test_guard_no_raise(self, simple_coalition):
        """CoalitionBoundary.guard() retourne l'exception sans lever."""
        exc = simple_coalition.guard(
            "max_tokens", 999_999,
            agent_id="rogue-agent",
            raise_exception=False,
        )
        assert exc is not None
        assert isinstance(exc, CoalitionBoundaryViolationError)

    def test_valid_guard_passes(self, simple_coalition):
        """Si la valeur est valide, guard() retourne None."""
        result = simple_coalition.guard(
            "max_tokens", 42_000,
            agent_id="nice-agent",
        )
        assert result is None

    def test_apply_valid(self, simple_coalition):
        """apply() modifie l'état si valide."""
        simple_coalition.apply("max_tokens", 75_000, agent_id="nice-agent")
        assert simple_coalition.get_view().get("max_tokens") == 75_000

    def test_apply_invalid(self, simple_coalition):
        """apply() lève si invalide et ne modifie PAS l'état."""
        old_value = simple_coalition.get_view().get("max_tokens")
        with pytest.raises(CoalitionBoundaryViolationError):
            simple_coalition.apply("max_tokens", -1, agent_id="rogue")
        # L'état n'a pas changé
        assert simple_coalition.get_view().get("max_tokens") == old_value


# ════════════════════════════════════════════════════════════════
# Option B — CoalitionWatchdog
# ════════════════════════════════════════════════════════════════

class TestOptionB_Watchdog:
    """Option B: log chirurgical + observation kernel."""

    def test_catch_violation_logs_and_observes(self, watchdog, capture_observer):
        """watchdog.catch() log et observe."""
        exc = CoalitionBoundaryViolationError(
            agent_id="test-agent",
            variable_name="max_tokens",
            attempted_value=999_999,
            constraint_description="max_tokens (int) max=100000",
        )
        severity = watchdog.catch(exc)
        assert severity == ViolationSeverity.VIOLATION
        # L'observation a été envoyée
        assert len(capture_observer.observations) == 1
        obs = capture_observer.observations[0]
        assert obs["domain"] == "coalition_watchdog"
        assert obs["content"]["event"] == "coalition_boundary_violation"
        assert obs["content"]["agent_id"] == "test-agent"

    def test_escalate_on_specific_variable(self, watchdog, capture_observer):
        """Les variables escaladées deviennent COUP_DETAT."""
        watchdog.escalate("max_tokens")
        exc = CoalitionBoundaryViolationError(
            agent_id="rogue",
            variable_name="max_tokens",
            attempted_value=999_999,
            constraint_description="max_tokens (int) max=100000",
        )
        severity = watchdog.catch(exc)
        assert severity == ViolationSeverity.COUP_DETAT
        obs = capture_observer.observations[0]
        assert obs["content"]["severity"] == "coup_detat"

    def test_non_escalated_variable(self, watchdog, capture_observer):
        """Les variables non escaladées restent VIOLATION."""
        exc = CoalitionBoundaryViolationError(
            agent_id="rogue",
            variable_name="ratio",
            attempted_value=2.0,
            constraint_description="ratio (float) max=1.0",
        )
        severity = watchdog.catch(exc)
        assert severity == ViolationSeverity.VIOLATION

    def test_stats_tracking(self, watchdog):
        """Les stats sont mises à jour à chaque violation."""
        exc1 = CoalitionBoundaryViolationError(
            agent_id="agent-a", variable_name="max_tokens",
            attempted_value=999_999, constraint_description="max=100000",
        )
        exc2 = CoalitionBoundaryViolationError(
            agent_id="agent-b", variable_name="model",
            attempted_value="gpt-4", constraint_description="allowed={a,b}",
        )
        watchdog.catch(exc1)
        watchdog.catch(exc2)

        stats = watchdog.stats.snapshot()
        assert stats["total_violations"] == 2
        assert stats["violations_by_agent"]["agent-a"] == 1
        assert stats["violations_by_agent"]["agent-b"] == 1
        assert stats["violations_by_variable"]["max_tokens"] == 1
        assert stats["violations_by_variable"]["model"] == 1

    def test_guarding_context_manager_passes(self, watchdog, simple_coalition):
        """Context manager guarding ne lève pas si pas de violation."""
        with watchdog.guarding(agent_id="test-agent"):
            pass  # Pas de violation → OK

    def test_guarding_context_manager_catches_and_relèves(self, watchdog, simple_coalition, capture_observer):
        """Context manager guarding intercepte, log, et re-lève."""
        with pytest.raises(CoalitionBoundaryViolationError):
            with watchdog.guarding(agent_id="rogue"):
                simple_coalition.guard("max_tokens", 999_999, agent_id="rogue")
        # L'observation a été envoyée
        assert len(capture_observer.observations) == 1

    def test_wraps_decorator(self, watchdog, simple_coalition):
        """Décorateur wraps intercepte les violations."""

        @watchdog.wraps
        def do_bad_thing():
            simple_coalition.guard("max_tokens", 999_999, agent_id="rogue")

        with pytest.raises(CoalitionBoundaryViolationError):
            do_bad_thing()

    def test_set_agent_id(self, watchdog):
        watchdog.set_agent_id("new-agent-id")
        assert watchdog.agent_id == "new-agent-id"

    def test_log_coalition_coup_detat_stderr(self):
        """log_coalition_coup_detat envoie bien dans stderr."""
        exc = CoalitionBoundaryViolationError(
            agent_id="rogue",
            variable_name="max_tokens",
            attempted_value=999_999,
            constraint_description="max_tokens (int) max=100000",
        )
        stderr_capture = StringIO()
        old_stderr, sys.stderr = sys.stderr, stderr_capture
        try:
            log_coalition_coup_detat(exc, to_stderr=True, to_logger=False)
        finally:
            sys.stderr = old_stderr

        output = stderr_capture.getvalue()
        assert "COUP D'ÉTAT ALGORITHMIQUE" in output
        assert "rogue" in output
        assert "max_tokens" in output

    def test_make_task_executor_watchdog(self, capture_observer, simple_coalition):
        """make_task_executor_watchdog crée un watchdog préconfiguré."""
        w = make_task_executor_watchdog(
            observer=capture_observer,
            coalition=simple_coalition,
            agent_id="hermes-forge",
        )
        assert w.agent_id == "hermes-forge"
        assert "max_tokens_per_task" in w._escalate_on  # escaladé par défaut
        assert "allowed_models" in w._escalate_on
        assert "session_compute_units" in w._escalate_on

    def test_null_observer_never_fails(self):
        """NullObserver ne lève jamais, même sans kernel."""
        obs = NullObserver()
        assert obs.observe("test", {}, "agent") is True

    def test_watchdog_stats_report(self, watchdog, capture_observer):
        """watchdog.report_stats() envoie un snapshot dans le kernel."""
        # Ajouter d'abord une violation
        exc = CoalitionBoundaryViolationError(
            agent_id="test-agent", variable_name="max_tokens",
            attempted_value=999_999, constraint_description="max=100000",
        )
        watchdog.catch(exc)

        # Puis reporter les stats
        stats = watchdog.report_stats()
        assert stats["total_violations"] == 1

        # Vérifier l'observation du rapport
        obs = capture_observer.observations[-1]
        assert obs["content"]["event"] == "watchdog_stats_report"


# ════════════════════════════════════════════════════════════════
# Tests avancés: variables immutables, cohérence inter-variables
# ════════════════════════════════════════════════════════════════

class TestAdvancedBoundary:
    """Tests des fonctionnalités avancées de CoalitionBoundary."""

    def test_immutable_variable_cannot_change(self, immortal_coalition):
        """Un variable immutable ne peut pas être modifiée après initialisation."""
        with pytest.raises(CoalitionBoundaryViolationError):
            immortal_coalition.apply("max_connections", 10, agent_id="agent")

    def test_declare_twice_raises(self, simple_coalition):
        """Déclarer deux fois la même variable lève une erreur."""
        with pytest.raises(ValueError) as exc:
            simple_coalition.declare(CoalitionVariable(
                name="max_tokens", var_type=int,
            ))
        assert "déjà déclarée" in str(exc.value)

    def test_initialize_unknown_variable_raises(self, simple_coalition):
        """Initialiser une variable non déclarée lève KeyError."""
        with pytest.raises(KeyError):
            simple_coalition.initialize("unknown_var", 42)

    def test_initialize_twice_raises(self, simple_coalition):
        """Initialiser deux fois la même variable lève ValueError."""
        with pytest.raises(ValueError):
            simple_coalition.initialize("max_tokens", 60_000)

    def test_coalition_view_snapshot(self, simple_coalition):
        """get_view().snapshot() retourne une copie de l'état."""
        view = simple_coalition.get_view()
        snapshot = view.snapshot()
        assert snapshot["max_tokens"] == 50_000
        # Modifier le snapshot ne doit pas affecter l'original
        snapshot["max_tokens"] = 0
        assert view.get("max_tokens") == 50_000

    def test_coalition_describe(self, simple_coalition):
        """describe() retourne la constitution lisible."""
        desc = simple_coalition.describe()
        assert "test_coalition" in desc
        assert "max_tokens" in desc
        assert "int" in desc
        assert "IMMUTABLE" not in desc  # Aucune var immutable dans simple_coalition

    def test_immortal_describe(self, immortal_coalition):
        """describe() montre les variables immutables."""
        desc = immortal_coalition.describe()
        assert "🔒" in desc  # Emoji cadenas pour immutable

    def test_view_contains(self, simple_coalition):
        """__contains__ sur CoalitionStateView."""
        view = simple_coalition.get_view()
        assert "max_tokens" in view
        assert "nonexistent" not in view

    def test_view_getitem(self, simple_coalition):
        """__getitem__ sur CoalitionStateView."""
        view = simple_coalition.get_view()
        assert view["max_tokens"] == 50_000

    def test_view_getitem_unknown(self, simple_coalition):
        """__getitem__ sur variable inconnue lève KeyError."""
        view = simple_coalition.get_view()
        with pytest.raises(KeyError):
            _ = view["nonexistent"]

    def test_view_describe(self, simple_coalition):
        """describe() sur la vue retourne l'état lisible."""
        desc = simple_coalition.get_view().describe()
        assert "max_tokens" in desc
        assert "50000" in desc

    def test_coherence_check(self):
        """Callback de cohérence inter-variables."""
        def check_ratio_must_be_lower_when_small_max(view: CoalitionStateView, proposed: Any) -> bool:
            max_tokens = view.get("max_tokens", 0)
            if max_tokens < 10_000 and proposed > 0.8:
                return False
            return True

        c = CoalitionBoundary(name="coherent")
        c.declare(CoalitionVariable(
            name="max_tokens", var_type=int, min_value=1, max_value=100_000,
        ))
        c.declare(CoalitionVariable(
            name="ratio", var_type=float, min_value=0.0, max_value=1.0,
            coherence_check=check_ratio_must_be_lower_when_small_max,
        ))
        c.initialize("max_tokens", 5_000)  # Petit budget
        c.initialize("ratio", 0.5)  # Ratio OK

        # Maintenant on essaie de mettre le ratio à 0.9 → doit échouer
        # parce que max_tokens < 10_000 et ratio > 0.8
        error = c.validate("ratio", 0.9)
        assert error is not None
        assert "cohérence inter-variables" in error

    def test_apply_updates_state(self, simple_coalition):
        """apply() met à jour l'état après validation."""
        simple_coalition.apply("max_tokens", 75_000, agent_id="test")
        assert simple_coalition.get_view()["max_tokens"] == 75_000
        simple_coalition.apply("model", "claude-3", agent_id="test")
        assert simple_coalition.get_view()["model"] == "claude-3"

    def test_apply_multiple_mutations(self, simple_coalition):
        """Plusieurs apply() successives fonctionnent."""
        simple_coalition.apply("ratio", 0.1, agent_id="test")
        simple_coalition.apply("ratio", 0.5, agent_id="test")
        simple_coalition.apply("ratio", 1.0, agent_id="test")
        assert simple_coalition.get_view()["ratio"] == 1.0


# ════════════════════════════════════════════════════════════════
# Tests de ViolationSeverity enum
# ════════════════════════════════════════════════════════════════

class TestViolationSeverity:
    def test_values(self):
        assert ViolationSeverity.WARNING.value == "warning"
        assert ViolationSeverity.VIOLATION.value == "violation"
        assert ViolationSeverity.COUP_DETAT.value == "coup_detat"

    def test_order(self):
        """Les sévérités sont dans l'ordre croissant de gravité."""
        severities = list(ViolationSeverity)
        assert severities == [
            ViolationSeverity.WARNING,
            ViolationSeverity.VIOLATION,
            ViolationSeverity.COUP_DETAT,
        ]