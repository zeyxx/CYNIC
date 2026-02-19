"""
CYNIC HumanStateModel — Live model of the human operator's state (T34)

Tracks signals about the human's cognitive and emotional state.
Updated by external events (session start, feedback, corrections, long gaps).
Used by Orchestrator to adapt judgment depth:
  LOW energy → REFLEX only (don't overload)
  HIGH focus → MACRO allowed (deep analysis welcome)

Dimensions (all 0-100, phi-bounded thresholds):
  energy:  overall cognitive energy (100 = fresh, 0 = exhausted)
  focus:   current focus level (100 = flow state)
  stress:  current stress level (100 = max stress)
  valence: mood / affect tone (100 = very positive, 0 = very negative)

Signals (what updates each dimension):
  session_started  → energy/focus mild reset (+5 each, capped at 100)
  feedback         → valence tracks rating; 5-star = +10 valence, 1-star = -10
  correction       → stress += 15 (human had to correct = friction = stress)
  idle_gap         → energy restores +10 per 10min gap (rest = recovery)
  burst_activity   → energy -= 10 per burst (3+ judgments in 60s)

Decay: energy/focus/valence decay 1pt per minute of activity (natural fatigue).

phi-derived thresholds:
  energy/focus >= WAG_MIN (61.8)   → organism at normal capacity
  energy/focus <  GROWL_MIN (38.2) → organism stressed → cap at MICRO
  energy/focus <  PHI_INV_3 (23.6) → organism exhausted → cap at REFLEX
"""
from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any, Dict, Optional

from cynic.core.phi import WAG_MIN, GROWL_MIN, PHI_INV_3

logger = logging.getLogger("cynic.core.human_state")

# Decay rate: 1 point per 60s of activity (physiological approximation)
_DECAY_RATE_PER_S = 1.0 / 60.0

# Inactivity threshold for "rest" recovery: 5 minutes
_REST_THRESHOLD_S = 300.0

# Energy restore per rest cycle (every 60s of inactivity)
_REST_RESTORE = 2.0


@dataclass
class HumanState:
    """Point-in-time snapshot of the human operator's estimated state."""
    energy:  float = 75.0   # [0, 100] cognitive energy
    focus:   float = 65.0   # [0, 100] focus level
    stress:  float = 20.0   # [0, 100] stress level
    valence: float = 65.0   # [0, 100] affect / mood tone

    updated_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        from cynic.core.phi import WAG_MIN, GROWL_MIN, PHI_INV_3
        lod_hint = "FULL"
        effective = min(self.energy, self.focus)
        if effective < PHI_INV_3 * 100:
            lod_hint = "REFLEX"
        elif effective < GROWL_MIN:
            lod_hint = "MICRO"
        return {
            "energy":     round(self.energy, 1),
            "focus":      round(self.focus, 1),
            "stress":     round(self.stress, 1),
            "valence":    round(self.valence, 1),
            "lod_hint":   lod_hint,
            "updated_at": round(self.updated_at, 3),
        }


class HumanStateModel:
    """
    Maintains a rolling estimate of the human's cognitive state.

    Wired to event bus by state.py:
      USER_FEEDBACK  → update valence / stress
      USER_CORRECTION → increase stress
      SDK_SESSION_STARTED → mild energy reset
      periodic decay via _apply_decay()
    """

    def __init__(self) -> None:
        self._state = HumanState()
        self._last_activity_ts: float = time.time()
        self._signal_count: int = 0

    # ── Signal receivers ───────────────────────────────────────────────────

    def on_session_started(self) -> None:
        """New session = human is fresh-ish (mild boost)."""
        self._apply_decay()
        self._state.energy  = min(100.0, self._state.energy + 5.0)
        self._state.focus   = min(100.0, self._state.focus  + 5.0)
        self._state.updated_at = time.time()
        self._touch()
        logger.debug("HumanStateModel: session_started → energy=%.1f focus=%.1f",
                     self._state.energy, self._state.focus)

    def on_feedback(self, rating: float) -> None:
        """User gave a rating (1-5). Positive = valence up; negative = stress up."""
        self._apply_decay()
        # Valence: 5-star=+10, 3-star=0, 1-star=-10 (centered at 3)
        delta_v = (rating - 3.0) / 2.0 * 10.0
        self._state.valence = max(0.0, min(100.0, self._state.valence + delta_v))
        # Stress: low ratings indicate frustration
        if rating <= 2:
            self._state.stress = min(100.0, self._state.stress + 10.0)
        elif rating >= 4:
            self._state.stress = max(0.0, self._state.stress - 5.0)
        self._state.updated_at = time.time()
        self._touch()
        self._signal_count += 1
        logger.debug("HumanStateModel: feedback=%.1f → valence=%.1f stress=%.1f",
                     rating, self._state.valence, self._state.stress)

    def on_correction(self) -> None:
        """Human corrected CYNIC (rating=1). Friction = stress signal."""
        self._apply_decay()
        self._state.stress = min(100.0, self._state.stress + 15.0)
        # Corrections also drain focus slightly
        self._state.focus  = max(0.0, self._state.focus - 5.0)
        self._state.updated_at = time.time()
        self._touch()
        logger.debug("HumanStateModel: correction → stress=%.1f focus=%.1f",
                     self._state.stress, self._state.focus)

    def on_activity(self) -> None:
        """Called on any human-triggered activity (judgment request, etc.)."""
        now = time.time()
        gap = now - self._last_activity_ts
        if gap > _REST_THRESHOLD_S:
            # Human was idle → rested → energy restored
            rest_cycles = gap / 60.0
            restore = min(rest_cycles * _REST_RESTORE, 20.0)
            self._state.energy  = min(100.0, self._state.energy  + restore)
            self._state.stress  = max(0.0,   self._state.stress  - restore * 0.5)
            logger.debug("HumanStateModel: rest gap %.0fs → energy restored +%.1f",
                         gap, restore)
        else:
            self._apply_decay()
        self._last_activity_ts = now
        self._state.updated_at = now

    # ── Snapshot ───────────────────────────────────────────────────────────

    def snapshot(self) -> Dict[str, Any]:
        self._apply_decay()
        return self._state.to_dict()

    @property
    def lod_hint(self) -> str:
        """Current LOD recommendation based on human state."""
        effective = min(self._state.energy, self._state.focus)
        if effective < PHI_INV_3 * 100:
            return "REFLEX"
        if effective < GROWL_MIN:
            return "MICRO"
        return "FULL"

    @property
    def energy(self) -> float:
        return self._state.energy

    @property
    def focus(self) -> float:
        return self._state.focus

    @property
    def stress(self) -> float:
        return self._state.stress

    # ── Private ────────────────────────────────────────────────────────────

    def _apply_decay(self) -> None:
        """Natural fatigue: energy/focus decay 1pt/min, stress decays slightly."""
        now     = time.time()
        elapsed = now - self._state.updated_at
        if elapsed < 1.0:
            return
        decay = elapsed * _DECAY_RATE_PER_S
        self._state.energy  = max(0.0, self._state.energy  - decay)
        self._state.focus   = max(0.0, self._state.focus   - decay)
        self._state.stress  = max(0.0, self._state.stress  - decay * 0.5)  # stress decays slower
        self._state.updated_at = now

    def _touch(self) -> None:
        self._last_activity_ts = time.time()
