"""Test Priority 4 Phase 3: Collection Immutability in UnifiedConsciousState

Verifies that:
1. dog_agreement_scores uses copy-on-write (no in-place mutation)
2. emergent_states uses copy-on-write (no in-place mutation)
3. activation_log stores MappingProxyType frozen entries
4. active_axioms tuple behavior is preserved
5. Validators and consensus calculations still work correctly
"""

from __future__ import annotations

from types import MappingProxyType

import pytest

from cynic.kernel.core.unified_state import UnifiedConsciousState


class TestDogAgreementScoresCopyOnWrite:
    """Test that dog_agreement_scores uses copy-on-write pattern."""

    def test_old_reference_unchanged_after_update(self):
        """Verify old dict reference is not mutated by update."""
        state = UnifiedConsciousState()
        state.dog_agreement_scores = {1: 0.5, 2: 0.7}
        old_ref = state.dog_agreement_scores

        # Update via method
        state.update_dog_agreement_score(1, 0.9)

        # Old reference unchanged
        assert old_ref[1] == 0.5
        # New reference in state
        assert state.dog_agreement_scores[1] == 0.9
        # Different dict instances
        assert old_ref is not state.dog_agreement_scores

    def test_updating_existing_key_creates_new_dict(self):
        """Verify updating an existing key creates a new dict."""
        state = UnifiedConsciousState()
        state.dog_agreement_scores = {5: 0.3}
        original_dict = state.dog_agreement_scores

        state.update_dog_agreement_score(5, 0.6)

        # New dict instance
        assert state.dog_agreement_scores is not original_dict
        # Value updated in new dict
        assert state.dog_agreement_scores[5] == 0.6

    def test_other_entries_preserved_in_new_dict(self):
        """Verify other entries are preserved when updating one key."""
        state = UnifiedConsciousState()
        state.dog_agreement_scores = {1: 0.5, 2: 0.7, 3: 0.8}

        state.update_dog_agreement_score(2, 0.9)

        # All keys present
        assert len(state.dog_agreement_scores) == 3
        # Updated key
        assert state.dog_agreement_scores[2] == 0.9
        # Other keys preserved
        assert state.dog_agreement_scores[1] == 0.5
        assert state.dog_agreement_scores[3] == 0.8

    def test_bounds_validation_raises_on_out_of_range(self):
        """Verify bounds validation still works."""
        state = UnifiedConsciousState()
        state.dog_agreement_scores = {}

        # Below 0.0
        with pytest.raises(ValueError, match="must be in \\[0.0, 1.0\\]"):
            state.update_dog_agreement_score(1, -0.1)

        # Above 1.0
        with pytest.raises(ValueError, match="must be in \\[0.0, 1.0\\]"):
            state.update_dog_agreement_score(1, 1.1)

    def test_boundary_values_accepted(self):
        """Verify boundary values 0.0 and 1.0 are accepted."""
        state = UnifiedConsciousState()
        state.dog_agreement_scores = {}

        state.update_dog_agreement_score(1, 0.0)
        assert state.dog_agreement_scores[1] == 0.0

        state.update_dog_agreement_score(2, 1.0)
        assert state.dog_agreement_scores[2] == 1.0


class TestEmergentStatesCopyOnWrite:
    """Test that emergent_states uses copy-on-write pattern."""

    def test_old_reference_unchanged_after_set(self):
        """Verify old dict reference is not mutated by set_emergent_state."""
        state = UnifiedConsciousState()
        state.emergent_states = {"state_a": True, "state_b": False}
        old_ref = state.emergent_states

        # Update via method
        state.set_emergent_state("state_a", False)

        # Old reference unchanged
        assert old_ref["state_a"] is True
        # New reference in state
        assert state.emergent_states["state_a"] is False
        # Different dict instances
        assert old_ref is not state.emergent_states

    def test_updating_existing_key_creates_new_dict(self):
        """Verify updating an existing key creates a new dict."""
        state = UnifiedConsciousState()
        state.emergent_states = {"emergency": True}
        original_dict = state.emergent_states

        state.set_emergent_state("emergency", False)

        # New dict instance
        assert state.emergent_states is not original_dict
        # Value updated in new dict
        assert state.emergent_states["emergency"] is False

    def test_other_entries_preserved_in_new_dict(self):
        """Verify other entries are preserved when updating one key."""
        state = UnifiedConsciousState()
        state.emergent_states = {"a": True, "b": False, "c": True}

        state.set_emergent_state("b", True)

        # All keys present
        assert len(state.emergent_states) == 3
        # Updated key
        assert state.emergent_states["b"] is True
        # Other keys preserved
        assert state.emergent_states["a"] is True
        assert state.emergent_states["c"] is True


class TestActivationLogMappingProxyType:
    """Test that activation_log stores frozen MappingProxyType entries."""

    def test_stored_entries_are_mapping_proxy_type(self):
        """Verify stored entries are MappingProxyType instances."""
        state = UnifiedConsciousState()
        log_entry = {"event": "test", "value": 42}

        state.log_activation(log_entry)

        assert len(state.activation_log) == 1
        assert isinstance(state.activation_log[0], MappingProxyType)

    def test_stored_entries_raise_on_write_attempt(self):
        """Verify stored entries cannot be modified."""
        state = UnifiedConsciousState()
        state.log_activation({"key": "value"})

        frozen_entry = state.activation_log[0]

        # Attempting to modify should raise TypeError
        with pytest.raises(TypeError):
            frozen_entry["key"] = "new_value"

        with pytest.raises(TypeError):
            frozen_entry["new_key"] = "new_value"

    def test_caller_mutations_dont_affect_stored_entry(self):
        """Verify caller's dict mutations don't leak into stored state."""
        state = UnifiedConsciousState()
        log_entry = {"event": "test", "counter": 0}

        state.log_activation(log_entry)

        # Caller mutates original dict
        log_entry["counter"] = 99
        log_entry["new_field"] = "injected"

        # Stored entry unchanged (snapshot via copy)
        assert state.activation_log[0]["counter"] == 0
        assert "new_field" not in state.activation_log[0]

    def test_multiple_entries_all_frozen(self):
        """Verify multiple log entries are all frozen."""
        state = UnifiedConsciousState()

        state.log_activation({"id": 1})
        state.log_activation({"id": 2})
        state.log_activation({"id": 3})

        assert len(state.activation_log) == 3

        # All are MappingProxyType
        for entry in state.activation_log:
            assert isinstance(entry, MappingProxyType)
            with pytest.raises(TypeError):
                entry["tamper"] = True

    def test_activation_log_tuple_grows_immutably(self):
        """Verify activation_log tuple itself is replaced (new instance)."""
        state = UnifiedConsciousState()
        original_tuple = state.activation_log

        state.log_activation({"event": "first"})
        first_tuple = state.activation_log

        state.log_activation({"event": "second"})
        second_tuple = state.activation_log

        # Each operation creates new tuple
        assert original_tuple is not first_tuple
        assert first_tuple is not second_tuple
        # Sizes grow
        assert len(original_tuple) == 0
        assert len(first_tuple) == 1
        assert len(second_tuple) == 2


class TestActiveAxiomsAlreadyCorrect:
    """Regression guard: active_axioms tuple behavior is preserved."""

    def test_active_axioms_is_tuple(self):
        """Verify active_axioms is a tuple."""
        state = UnifiedConsciousState()
        assert isinstance(state.active_axioms, tuple)

    def test_add_axiom_creates_new_tuple(self):
        """Verify add_axiom creates a new tuple, not appending in-place."""
        state = UnifiedConsciousState()
        original_tuple = state.active_axioms

        state.add_axiom("FIDELITY")
        first_tuple = state.active_axioms

        # New tuple instance
        assert original_tuple is not first_tuple
        assert len(original_tuple) == 0
        assert len(first_tuple) == 1
        assert first_tuple[0] == "FIDELITY"

    def test_add_axiom_is_idempotent(self):
        """Verify add_axiom doesn't add duplicates."""
        state = UnifiedConsciousState()

        state.add_axiom("PHI")
        state.add_axiom("PHI")
        state.add_axiom("VERIFY")
        state.add_axiom("PHI")

        assert len(state.active_axioms) == 2
        assert "PHI" in state.active_axioms
        assert "VERIFY" in state.active_axioms


class TestValidatorsPreservedAfterPhase3:
    """Verify field validators still work after copy-on-write changes."""

    def test_field_validator_rejects_out_of_bounds_at_construction(self):
        """Verify @field_validator rejects bad scores at model construction."""
        # Out of bounds score at construction should be caught by validator
        with pytest.raises(ValueError, match="must be in \\[0.0, 1.0\\]"):
            UnifiedConsciousState(dog_agreement_scores={1: 1.5})

        with pytest.raises(ValueError, match="must be in \\[0.0, 1.0\\]"):
            UnifiedConsciousState(dog_agreement_scores={1: -0.5})

    def test_whole_dict_replacement_still_works(self):
        """Verify whole-dict replacement pattern (from test_unified_state.py) still works."""
        state = UnifiedConsciousState()

        # Replace entire dict with valid scores
        state.dog_agreement_scores = {1: 0.5, 2: 0.7, 3: 0.9}

        assert state.dog_agreement_scores[1] == 0.5
        assert state.dog_agreement_scores[2] == 0.7
        assert state.dog_agreement_scores[3] == 0.9

        # Can also replace with another valid dict
        state.dog_agreement_scores = {10: 0.2, 20: 0.8}
        assert state.dog_agreement_scores[10] == 0.2
        assert state.dog_agreement_scores[20] == 0.8
        assert 1 not in state.dog_agreement_scores

    def test_get_consensus_score_works_after_copy_on_write_updates(self):
        """Verify get_consensus_score() works correctly with copy-on-write updates."""
        state = UnifiedConsciousState()

        # Empty consensus score
        assert state.get_consensus_score() == 0.0

        # Add some scores via copy-on-write
        state.update_dog_agreement_score(1, 0.6)
        state.update_dog_agreement_score(2, 0.8)
        state.update_dog_agreement_score(3, 0.4)

        # Consensus is average
        expected = (0.6 + 0.8 + 0.4) / 3
        assert state.get_consensus_score() == pytest.approx(expected)

        # Update one score
        state.update_dog_agreement_score(2, 0.2)

        expected = (0.6 + 0.2 + 0.4) / 3
        assert state.get_consensus_score() == pytest.approx(expected)
