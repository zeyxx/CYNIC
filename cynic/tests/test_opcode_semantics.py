"""Tests for opcode_semantics.py — formal CISA specification."""

import pytest
from cynic.core.opcode_semantics import (
    OpcodeSpec,
    StorageTier,
    OPCODE_REGISTRY,
    OPCODE_NAMES,
    get_opcode_spec,
    verify_state_transition,
    opcodes_for_level,
    all_opcodes_documented,
    PERCEIVE_SPEC,
    JUDGE_SPEC,
    DECIDE_SPEC,
    ACT_SPEC,
    LEARN_SPEC,
    ACCOUNT_SPEC,
    EMERGE_SPEC,
)
from cynic.core.consciousness import ConsciousnessLevel


class TestOpcodeSpecStructure:
    """Test OpcodeSpec dataclass structure and methods."""

    def test_opcode_spec_has_required_fields(self):
        """All OpcodeSpec instances have required fields."""
        for name, spec in OPCODE_REGISTRY.items():
            assert hasattr(spec, "name")
            assert hasattr(spec, "preconditions")
            assert hasattr(spec, "postconditions")
            assert hasattr(spec, "state_transitions")
            assert hasattr(spec, "storage_tiers")
            assert hasattr(spec, "cost_usd")
            assert hasattr(spec, "consciousness_gates")
            assert hasattr(spec, "description")

    def test_opcode_spec_preconditions_are_lists_of_strings(self):
        """All preconditions are non-empty lists of strings."""
        for name, spec in OPCODE_REGISTRY.items():
            assert isinstance(spec.preconditions, list)
            assert len(spec.preconditions) > 0
            for precond in spec.preconditions:
                assert isinstance(precond, str), f"{name}: precondition not string"

    def test_opcode_spec_postconditions_are_lists_of_strings(self):
        """All postconditions are non-empty lists of strings."""
        for name, spec in OPCODE_REGISTRY.items():
            assert isinstance(spec.postconditions, list)
            assert len(spec.postconditions) > 0
            for postcond in spec.postconditions:
                assert isinstance(postcond, str), f"{name}: postcondition not string"

    def test_opcode_spec_state_transitions_are_lists_of_strings(self):
        """All state_transitions are non-empty lists of strings."""
        for name, spec in OPCODE_REGISTRY.items():
            assert isinstance(spec.state_transitions, list)
            assert len(spec.state_transitions) > 0
            for transition in spec.state_transitions:
                assert isinstance(transition, str), f"{name}: state_transition not string"

    def test_opcode_spec_storage_tiers_are_valid(self):
        """All storage_tiers are valid StorageTier enums."""
        for name, spec in OPCODE_REGISTRY.items():
            assert isinstance(spec.storage_tiers, list)
            assert len(spec.storage_tiers) > 0
            for tier in spec.storage_tiers:
                assert isinstance(tier, StorageTier), f"{name}: invalid StorageTier"

    def test_opcode_spec_cost_usd_is_callable_or_float(self):
        """All cost_usd are either float or callable."""
        for name, spec in OPCODE_REGISTRY.items():
            assert callable(spec.cost_usd) or isinstance(spec.cost_usd, float)

    def test_opcode_spec_consciousness_gates_complete(self):
        """All consciousness_gates have entries for expected levels."""
        expected_levels = {
            ConsciousnessLevel.REFLEX,
            ConsciousnessLevel.MICRO,
            ConsciousnessLevel.MACRO,
            ConsciousnessLevel.META,
        }
        for name, spec in OPCODE_REGISTRY.items():
            assert isinstance(spec.consciousness_gates, dict)
            for level in expected_levels:
                assert level in spec.consciousness_gates, f"{name}: missing gate for {level}"


class TestOpcodeGating:
    """Test consciousness level gating per opcode."""

    def test_perceive_all_levels_permitted(self):
        """PERCEIVE opcode permitted at all consciousness levels."""
        spec = get_opcode_spec("PERCEIVE")
        assert spec.gate_for_level(ConsciousnessLevel.REFLEX)
        assert spec.gate_for_level(ConsciousnessLevel.MICRO)
        assert spec.gate_for_level(ConsciousnessLevel.MACRO)
        assert spec.gate_for_level(ConsciousnessLevel.META)

    def test_judge_all_levels_permitted(self):
        """JUDGE opcode permitted at all consciousness levels."""
        spec = get_opcode_spec("JUDGE")
        assert spec.gate_for_level(ConsciousnessLevel.REFLEX)
        assert spec.gate_for_level(ConsciousnessLevel.MICRO)
        assert spec.gate_for_level(ConsciousnessLevel.MACRO)
        assert spec.gate_for_level(ConsciousnessLevel.META)

    def test_decide_macro_and_meta_only(self):
        """DECIDE opcode permitted only at MACRO and META levels."""
        spec = get_opcode_spec("DECIDE")
        assert not spec.gate_for_level(ConsciousnessLevel.REFLEX)
        assert not spec.gate_for_level(ConsciousnessLevel.MICRO)
        assert spec.gate_for_level(ConsciousnessLevel.MACRO)
        assert spec.gate_for_level(ConsciousnessLevel.META)

    def test_act_macro_and_meta_only(self):
        """ACT opcode permitted only at MACRO and META levels."""
        spec = get_opcode_spec("ACT")
        assert not spec.gate_for_level(ConsciousnessLevel.REFLEX)
        assert not spec.gate_for_level(ConsciousnessLevel.MICRO)
        assert spec.gate_for_level(ConsciousnessLevel.MACRO)
        assert spec.gate_for_level(ConsciousnessLevel.META)

    def test_learn_macro_and_meta_only(self):
        """LEARN opcode permitted only at MACRO and META levels."""
        spec = get_opcode_spec("LEARN")
        assert not spec.gate_for_level(ConsciousnessLevel.REFLEX)
        assert not spec.gate_for_level(ConsciousnessLevel.MICRO)
        assert spec.gate_for_level(ConsciousnessLevel.MACRO)
        assert spec.gate_for_level(ConsciousnessLevel.META)

    def test_account_macro_and_meta_only(self):
        """ACCOUNT opcode permitted only at MACRO and META levels."""
        spec = get_opcode_spec("ACCOUNT")
        assert not spec.gate_for_level(ConsciousnessLevel.REFLEX)
        assert not spec.gate_for_level(ConsciousnessLevel.MICRO)
        assert spec.gate_for_level(ConsciousnessLevel.MACRO)
        assert spec.gate_for_level(ConsciousnessLevel.META)

    def test_emerge_meta_only(self):
        """EMERGE opcode permitted only at META level."""
        spec = get_opcode_spec("EMERGE")
        assert not spec.gate_for_level(ConsciousnessLevel.REFLEX)
        assert not spec.gate_for_level(ConsciousnessLevel.MICRO)
        assert not spec.gate_for_level(ConsciousnessLevel.MACRO)
        assert spec.gate_for_level(ConsciousnessLevel.META)


class TestStateTransitions:
    """Test opcode state transition graph."""

    def test_7step_cycle_valid(self):
        """7-step cycle follows valid transitions: PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE→PERCEIVE."""
        cycle = ["PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE", "PERCEIVE"]
        for i in range(len(cycle) - 1):
            from_opcode = cycle[i]
            to_opcode = cycle[i + 1]
            assert verify_state_transition(from_opcode, to_opcode), \
                f"Invalid transition: {from_opcode} → {to_opcode}"

    def test_perceive_only_transitions_to_judge(self):
        """PERCEIVE only transitions to JUDGE."""
        spec = get_opcode_spec("PERCEIVE")
        assert spec.state_transitions == ["JUDGE"]

    def test_judge_only_transitions_to_decide(self):
        """JUDGE only transitions to DECIDE."""
        spec = get_opcode_spec("JUDGE")
        assert spec.state_transitions == ["DECIDE"]

    def test_decide_only_transitions_to_act(self):
        """DECIDE only transitions to ACT (or loops back to PERCEIVE on rejection)."""
        spec = get_opcode_spec("DECIDE")
        assert "ACT" in spec.state_transitions

    def test_act_only_transitions_to_learn(self):
        """ACT only transitions to LEARN."""
        spec = get_opcode_spec("ACT")
        assert spec.state_transitions == ["LEARN"]

    def test_learn_only_transitions_to_account(self):
        """LEARN only transitions to ACCOUNT."""
        spec = get_opcode_spec("LEARN")
        assert spec.state_transitions == ["ACCOUNT"]

    def test_account_only_transitions_to_emerge(self):
        """ACCOUNT only transitions to EMERGE."""
        spec = get_opcode_spec("ACCOUNT")
        assert spec.state_transitions == ["EMERGE"]

    def test_emerge_transitions_to_perceive(self):
        """EMERGE transitions back to PERCEIVE (loop)."""
        spec = get_opcode_spec("EMERGE")
        assert spec.state_transitions == ["PERCEIVE"]

    def test_verify_state_transition_invalid_opcode(self):
        """verify_state_transition returns False for invalid opcodes."""
        assert not verify_state_transition("INVALID", "JUDGE")
        assert not verify_state_transition("PERCEIVE", "INVALID")

    def test_verify_state_transition_invalid_path(self):
        """verify_state_transition returns False for invalid transitions."""
        assert not verify_state_transition("PERCEIVE", "ACT")
        assert not verify_state_transition("JUDGE", "LEARN")
        assert not verify_state_transition("ACT", "PERCEIVE")


class TestOpcodeRegistry:
    """Test opcode registry integrity."""

    def test_all_7_opcodes_documented(self):
        """All 7 required opcodes are documented."""
        assert all_opcodes_documented()
        assert len(OPCODE_REGISTRY) == 7
        assert len(OPCODE_NAMES) == 7

    def test_registry_has_exact_required_opcodes(self):
        """Registry contains exactly the 7 required opcodes."""
        expected = {"PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE"}
        assert set(OPCODE_REGISTRY.keys()) == expected

    def test_registry_matches_names_list(self):
        """OPCODE_REGISTRY keys match OPCODE_NAMES."""
        assert set(OPCODE_REGISTRY.keys()) == set(OPCODE_NAMES)

    def test_get_opcode_spec_case_insensitive(self):
        """get_opcode_spec is case-insensitive."""
        assert get_opcode_spec("perceive") == get_opcode_spec("PERCEIVE")
        assert get_opcode_spec("Judge") == get_opcode_spec("JUDGE")

    def test_get_opcode_spec_invalid_returns_none(self):
        """get_opcode_spec returns None for invalid names."""
        assert get_opcode_spec("INVALID") is None
        assert get_opcode_spec("") is None


class TestOpcodesCostModel:
    """Test opcode cost models."""

    def test_perceive_fixed_cost(self):
        """PERCEIVE has fixed cost of $0.001."""
        spec = get_opcode_spec("PERCEIVE")
        assert spec.estimated_cost() == 0.001

    def test_decide_zero_cost(self):
        """DECIDE has zero cost."""
        spec = get_opcode_spec("DECIDE")
        assert spec.estimated_cost() == 0.0

    def test_learn_fixed_cost(self):
        """LEARN has fixed cost of $0.01."""
        spec = get_opcode_spec("LEARN")
        assert spec.estimated_cost() == 0.01

    def test_account_zero_cost(self):
        """ACCOUNT has zero cost."""
        spec = get_opcode_spec("ACCOUNT")
        assert spec.estimated_cost() == 0.0

    def test_emerge_fixed_cost(self):
        """EMERGE has fixed cost of $0.50."""
        spec = get_opcode_spec("EMERGE")
        assert spec.estimated_cost() == 0.50

    def test_judge_callable_cost(self):
        """JUDGE cost is callable (varies by level)."""
        spec = get_opcode_spec("JUDGE")
        assert callable(spec.cost_usd)
        # Cost model requires a level parameter, so we don't call estimated_cost()
        # Just verify the lambda works
        assert spec.cost_usd("L1") == 2.50
        assert spec.cost_usd("L3") == 0.02
        assert spec.cost_usd("L4") == 5.00

    def test_act_callable_cost(self):
        """ACT cost is callable."""
        spec = get_opcode_spec("ACT")
        assert callable(spec.cost_usd)


class TestOpcodeStorageTiers:
    """Test opcode storage tier routing."""

    def test_perceive_hot_only(self):
        """PERCEIVE writes to HOT tier only."""
        spec = get_opcode_spec("PERCEIVE")
        assert spec.storage_tiers == [StorageTier.HOT]

    def test_judge_multi_tier(self):
        """JUDGE writes to HOT, WARM, and COLD tiers."""
        spec = get_opcode_spec("JUDGE")
        assert set(spec.storage_tiers) == {StorageTier.HOT, StorageTier.WARM, StorageTier.COLD}

    def test_decide_warm_only(self):
        """DECIDE writes to WARM tier only."""
        spec = get_opcode_spec("DECIDE")
        assert spec.storage_tiers == [StorageTier.WARM]

    def test_act_cold_only(self):
        """ACT writes to COLD tier only."""
        spec = get_opcode_spec("ACT")
        assert spec.storage_tiers == [StorageTier.COLD]

    def test_learn_warm_only(self):
        """LEARN writes to WARM tier only."""
        spec = get_opcode_spec("LEARN")
        assert spec.storage_tiers == [StorageTier.WARM]

    def test_account_cold_only(self):
        """ACCOUNT writes to COLD tier only."""
        spec = get_opcode_spec("ACCOUNT")
        assert spec.storage_tiers == [StorageTier.COLD]

    def test_emerge_warm_and_cold(self):
        """EMERGE writes to WARM and COLD tiers."""
        spec = get_opcode_spec("EMERGE")
        assert set(spec.storage_tiers) == {StorageTier.WARM, StorageTier.COLD}


class TestOpcodeNamingAndDescriptions:
    """Test opcode names and descriptions."""

    def test_all_opcodes_have_name(self):
        """All opcodes have name field matching their key."""
        for key, spec in OPCODE_REGISTRY.items():
            assert spec.name == key

    def test_all_opcodes_have_description(self):
        """All opcodes have non-empty description."""
        for key, spec in OPCODE_REGISTRY.items():
            assert isinstance(spec.description, str)
            assert len(spec.description) > 0

    def test_opcode_names_uppercase(self):
        """All opcode names are uppercase."""
        for name in OPCODE_NAMES:
            assert name.isupper()


class TestOpcodesConcatenation:
    """Test consistency across all opcode specifications."""

    def test_consciousness_levels_enum_consistency(self):
        """All consciousness level gates use valid ConsciousnessLevel enum values."""
        valid_levels = {
            ConsciousnessLevel.REFLEX,
            ConsciousnessLevel.MICRO,
            ConsciousnessLevel.MACRO,
            ConsciousnessLevel.META,
        }
        for name, spec in OPCODE_REGISTRY.items():
            for level in spec.consciousness_gates.keys():
                assert level in valid_levels, f"{name}: invalid consciousness level"

    def test_state_transitions_reference_valid_opcodes(self):
        """All state_transitions reference valid opcodes in registry."""
        for from_name, from_spec in OPCODE_REGISTRY.items():
            for to_name in from_spec.state_transitions:
                assert to_name in OPCODE_REGISTRY, \
                    f"{from_name} → {to_name}: {to_name} not in registry"

    def test_storage_tiers_enum_consistency(self):
        """All storage tiers are valid StorageTier enum values."""
        valid_tiers = {StorageTier.HOT, StorageTier.WARM, StorageTier.COLD, StorageTier.FROZEN}
        for name, spec in OPCODE_REGISTRY.items():
            for tier in spec.storage_tiers:
                assert tier in valid_tiers, f"{name}: invalid StorageTier"


class TestOpcodesForLevel:
    """Test opcodes_for_level helper function."""

    def test_reflex_level_permits_perceive_and_judge(self):
        """L3 REFLEX permits PERCEIVE and JUDGE opcodes."""
        reflex_opcodes = opcodes_for_level(ConsciousnessLevel.REFLEX)
        assert "PERCEIVE" in reflex_opcodes
        assert "JUDGE" in reflex_opcodes
        assert "DECIDE" not in reflex_opcodes
        assert "ACT" not in reflex_opcodes

    def test_micro_level_permits_perceive_and_judge(self):
        """L2 MICRO permits PERCEIVE and JUDGE opcodes."""
        micro_opcodes = opcodes_for_level(ConsciousnessLevel.MICRO)
        assert "PERCEIVE" in micro_opcodes
        assert "JUDGE" in micro_opcodes
        assert "DECIDE" not in micro_opcodes

    def test_macro_level_permits_full_cycle(self):
        """L1 MACRO permits all opcodes except EMERGE."""
        macro_opcodes = opcodes_for_level(ConsciousnessLevel.MACRO)
        assert "PERCEIVE" in macro_opcodes
        assert "JUDGE" in macro_opcodes
        assert "DECIDE" in macro_opcodes
        assert "ACT" in macro_opcodes
        assert "LEARN" in macro_opcodes
        assert "ACCOUNT" in macro_opcodes
        assert "EMERGE" not in macro_opcodes

    def test_meta_level_permits_all_opcodes(self):
        """L4 META permits all opcodes."""
        meta_opcodes = opcodes_for_level(ConsciousnessLevel.META)
        assert set(meta_opcodes) == set(OPCODE_NAMES)


class TestIndividualOpcodeSpecs:
    """Test specific opcode specifications match expectations."""

    def test_perceive_spec_complete(self):
        """PERCEIVE spec has complete specification."""
        assert PERCEIVE_SPEC.name == "PERCEIVE"
        assert len(PERCEIVE_SPEC.preconditions) >= 2
        assert len(PERCEIVE_SPEC.postconditions) >= 3

    def test_judge_spec_complete(self):
        """JUDGE spec has complete specification."""
        assert JUDGE_SPEC.name == "JUDGE"
        assert len(JUDGE_SPEC.preconditions) >= 3
        assert len(JUDGE_SPEC.postconditions) >= 5

    def test_decide_spec_complete(self):
        """DECIDE spec has complete specification."""
        assert DECIDE_SPEC.name == "DECIDE"
        assert len(DECIDE_SPEC.preconditions) >= 3
        assert len(DECIDE_SPEC.postconditions) >= 3

    def test_act_spec_complete(self):
        """ACT spec has complete specification."""
        assert ACT_SPEC.name == "ACT"
        assert len(ACT_SPEC.preconditions) >= 3
        assert len(ACT_SPEC.postconditions) >= 3

    def test_learn_spec_complete(self):
        """LEARN spec has complete specification."""
        assert LEARN_SPEC.name == "LEARN"
        assert len(LEARN_SPEC.preconditions) >= 2
        assert len(LEARN_SPEC.postconditions) >= 3

    def test_account_spec_complete(self):
        """ACCOUNT spec has complete specification."""
        assert ACCOUNT_SPEC.name == "ACCOUNT"
        assert len(ACCOUNT_SPEC.preconditions) >= 2
        assert len(ACCOUNT_SPEC.postconditions) >= 3

    def test_emerge_spec_complete(self):
        """EMERGE spec has complete specification."""
        assert EMERGE_SPEC.name == "EMERGE"
        assert len(EMERGE_SPEC.preconditions) >= 2
        assert len(EMERGE_SPEC.postconditions) >= 4
