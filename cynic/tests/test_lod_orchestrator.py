"""
Tests: B2 fix — LOD cap enforced on explicit level + L2→L1 escalation.

Validates _apply_lod_cap() as the single enforcement point for:
  1. Explicit level: run(cell, level=MACRO) when LOD=EMERGENCY → REFLEX
  2. L2→L1 escalation: _cycle_micro → _cycle_macro when LOD=EMERGENCY → blocked
"""
from __future__ import annotations

import pytest
from unittest.mock import MagicMock

from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
from cynic.cognition.cortex.lod import SurvivalLOD, LODController
from cynic.core.consciousness import ConsciousnessLevel


# ── Helpers ──────────────────────────────────────────────────────────────────

def _make_orchestrator() -> JudgeOrchestrator:
    """Minimal JudgeOrchestrator for unit-testing _apply_lod_cap."""
    dogs = {}
    axiom_arch = MagicMock()
    cynic_dog = MagicMock()
    orch = JudgeOrchestrator(dogs=dogs, axiom_arch=axiom_arch, cynic_dog=cynic_dog)
    return orch


def _lod_at(level: SurvivalLOD) -> LODController:
    """Build an LODController forced to a specific LOD level."""
    ctrl = LODController()
    ctrl.force(level)
    return ctrl


# ═══════════════════════════════════════════════════════════════════════════
# _apply_lod_cap — no lod_controller
# ═══════════════════════════════════════════════════════════════════════════

class TestApplyLodCapNoController:

    def test_no_controller_macro_passthrough(self):
        """No lod_controller → always returns the level unchanged."""
        orch = _make_orchestrator()
        assert orch.lod_controller is None
        result = orch._apply_lod_cap(ConsciousnessLevel.MACRO)
        assert result == ConsciousnessLevel.MACRO

    def test_no_controller_micro_passthrough(self):
        orch = _make_orchestrator()
        assert orch._apply_lod_cap(ConsciousnessLevel.MICRO) == ConsciousnessLevel.MICRO

    def test_no_controller_reflex_passthrough(self):
        orch = _make_orchestrator()
        assert orch._apply_lod_cap(ConsciousnessLevel.REFLEX) == ConsciousnessLevel.REFLEX


# ═══════════════════════════════════════════════════════════════════════════
# _apply_lod_cap — FULL LOD (no restrictions)
# ═══════════════════════════════════════════════════════════════════════════

class TestApplyLodCapFull:

    def test_full_allows_macro(self):
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.FULL)
        assert orch._apply_lod_cap(ConsciousnessLevel.MACRO) == ConsciousnessLevel.MACRO

    def test_full_allows_micro(self):
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.FULL)
        assert orch._apply_lod_cap(ConsciousnessLevel.MICRO) == ConsciousnessLevel.MICRO

    def test_full_allows_reflex(self):
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.FULL)
        assert orch._apply_lod_cap(ConsciousnessLevel.REFLEX) == ConsciousnessLevel.REFLEX


# ═══════════════════════════════════════════════════════════════════════════
# _apply_lod_cap — REDUCED LOD (cap at MICRO)
# ═══════════════════════════════════════════════════════════════════════════

class TestApplyLodCapReduced:

    def test_reduced_caps_macro_to_micro(self):
        """REDUCED LOD: MACRO → MICRO."""
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.REDUCED)
        assert orch._apply_lod_cap(ConsciousnessLevel.MACRO) == ConsciousnessLevel.MICRO

    def test_reduced_allows_micro(self):
        """REDUCED LOD: MICRO → MICRO (already at cap)."""
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.REDUCED)
        assert orch._apply_lod_cap(ConsciousnessLevel.MICRO) == ConsciousnessLevel.MICRO

    def test_reduced_allows_reflex(self):
        """REDUCED LOD: REFLEX → REFLEX (already below cap)."""
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.REDUCED)
        assert orch._apply_lod_cap(ConsciousnessLevel.REFLEX) == ConsciousnessLevel.REFLEX


# ═══════════════════════════════════════════════════════════════════════════
# _apply_lod_cap — EMERGENCY LOD (cap at REFLEX)
# ═══════════════════════════════════════════════════════════════════════════

class TestApplyLodCapEmergency:

    def test_emergency_caps_macro_to_reflex(self):
        """EMERGENCY LOD: MACRO → REFLEX."""
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.EMERGENCY)
        assert orch._apply_lod_cap(ConsciousnessLevel.MACRO) == ConsciousnessLevel.REFLEX

    def test_emergency_caps_micro_to_reflex(self):
        """EMERGENCY LOD: MICRO → REFLEX."""
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.EMERGENCY)
        assert orch._apply_lod_cap(ConsciousnessLevel.MICRO) == ConsciousnessLevel.REFLEX

    def test_emergency_keeps_reflex(self):
        """EMERGENCY LOD: REFLEX → REFLEX (already at floor)."""
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.EMERGENCY)
        assert orch._apply_lod_cap(ConsciousnessLevel.REFLEX) == ConsciousnessLevel.REFLEX


# ═══════════════════════════════════════════════════════════════════════════
# _apply_lod_cap — MINIMAL LOD (also caps at REFLEX — MINIMAL >= EMERGENCY)
# ═══════════════════════════════════════════════════════════════════════════

class TestApplyLodCapMinimal:

    def test_minimal_caps_macro_to_reflex(self):
        """MINIMAL LOD: MACRO → REFLEX (MINIMAL >= EMERGENCY check)."""
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.MINIMAL)
        assert orch._apply_lod_cap(ConsciousnessLevel.MACRO) == ConsciousnessLevel.REFLEX

    def test_minimal_caps_micro_to_reflex(self):
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.MINIMAL)
        assert orch._apply_lod_cap(ConsciousnessLevel.MICRO) == ConsciousnessLevel.REFLEX

    def test_minimal_keeps_reflex(self):
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(SurvivalLOD.MINIMAL)
        assert orch._apply_lod_cap(ConsciousnessLevel.REFLEX) == ConsciousnessLevel.REFLEX


# ═══════════════════════════════════════════════════════════════════════════
# Idempotency — applying cap twice equals applying it once
# ═══════════════════════════════════════════════════════════════════════════

class TestApplyLodCapIdempotent:

    @pytest.mark.parametrize("lod", [
        SurvivalLOD.FULL, SurvivalLOD.REDUCED,
        SurvivalLOD.EMERGENCY, SurvivalLOD.MINIMAL,
    ])
    @pytest.mark.parametrize("level", [
        ConsciousnessLevel.REFLEX, ConsciousnessLevel.MICRO, ConsciousnessLevel.MACRO,
    ])
    def test_double_cap_equals_single(self, lod, level):
        """_apply_lod_cap is idempotent: cap(cap(x)) == cap(x)."""
        orch = _make_orchestrator()
        orch.lod_controller = _lod_at(lod)
        once = orch._apply_lod_cap(level)
        twice = orch._apply_lod_cap(once)
        assert once == twice
