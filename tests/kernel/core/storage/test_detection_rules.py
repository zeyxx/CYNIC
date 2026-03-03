"""
Detection Rules Tests — Kill Chain rule coverage and registry management
Tests Rule base class, RuleRegistry, RuleExecutor, and all 18+ kill chain rules
"""

import pytest

from cynic.kernel.core.storage.detection_rules import (
    RuleRegistry,
    RuleExecutor,
    Stage1_APIScanning,
    Stage2_SuspiciousProposal,
    Stage3_LargeVotingBloc,
    Stage4_ConsensusManipulation,
    Stage5_PersistentActor,
    Stage6_CoordinatedVoting,
    Stage7_MaliciousExecution,
    NewActorHighValueProposal,
    MultiActorCoordination,
    ProposalValueExplosion,
    VotingVelocitySpike,
    GovernanceParameterChange,
    TreasuryAddressChange,
    AnomalousConsensusVariance,
    RapidProposalCreation,
    BotVotingPattern,
    MissingJustification,
)


# ============================================================================
# FIXTURES
# ============================================================================


@pytest.fixture
def registry():
    """Create rule registry."""
    return RuleRegistry()


@pytest.fixture
def populated_registry():
    """Create registry with all default rules."""
    registry = RuleRegistry()
    rules = [
        Stage1_APIScanning("STAGE_1_API_SCANNING", "Reconnaissance", "MEDIUM"),
        Stage2_SuspiciousProposal("STAGE_2_WEAPONIZATION", "Weaponization", "HIGH"),
        Stage3_LargeVotingBloc("STAGE_3_VOTING_BLOC", "Delivery", "HIGH"),
        Stage4_ConsensusManipulation("STAGE_4_CONSENSUS", "Exploitation", "CRITICAL"),
        Stage5_PersistentActor("STAGE_5_PERSISTENCE", "Installation", "HIGH"),
        Stage6_CoordinatedVoting("STAGE_6_COORDINATION", "C2", "CRITICAL"),
        Stage7_MaliciousExecution("STAGE_7_EXECUTION", "Actions", "CRITICAL"),
        NewActorHighValueProposal("NEW_ACTOR_HIGH_VALUE", "Weaponization", "HIGH"),
        MultiActorCoordination("MULTI_ACTOR_COORD", "Delivery", "HIGH"),
        ProposalValueExplosion("PROPOSAL_VALUE_EXPLOSION", "Weaponization", "CRITICAL"),
        VotingVelocitySpike("VOTING_VELOCITY_SPIKE", "Reconnaissance", "HIGH"),
        GovernanceParameterChange("GOVERNANCE_PARAM_CHANGE", "Exploitation", "CRITICAL"),
        TreasuryAddressChange("TREASURY_ADDRESS_CHANGE", "Actions", "CRITICAL"),
        AnomalousConsensusVariance("ANOMALOUS_CONSENSUS", "Exploitation", "HIGH"),
        RapidProposalCreation("RAPID_PROPOSAL_CREATION", "Weaponization", "HIGH"),
        BotVotingPattern("BOT_VOTING_PATTERN", "C2", "HIGH"),
        MissingJustification("MISSING_JUSTIFICATION", "C2", "MEDIUM"),
    ]
    for rule in rules:
        registry.register(rule)
    return registry


@pytest.fixture
def executor(populated_registry):
    """Create rule executor."""
    return RuleExecutor(populated_registry)


# ============================================================================
# RULE REGISTRY TESTS
# ============================================================================


def test_registry_empty():
    """Test empty registry."""
    registry = RuleRegistry()
    assert len(registry.get_active_rules()) == 0


def test_registry_register_rule():
    """Test rule registration."""
    registry = RuleRegistry()
    rule = Stage1_APIScanning("TEST_RULE", "Reconnaissance", "LOW")
    registry.register(rule)

    assert rule in registry.get_active_rules()


def test_registry_disable_rule():
    """Test disabling a rule."""
    registry = RuleRegistry()
    rule = Stage1_APIScanning("TEST_RULE", "Reconnaissance", "LOW")
    registry.register(rule)
    registry.disable("TEST_RULE")

    assert rule not in registry.get_active_rules()


def test_registry_enable_rule():
    """Test re-enabling a rule."""
    registry = RuleRegistry()
    rule = Stage1_APIScanning("TEST_RULE", "Reconnaissance", "LOW")
    registry.register(rule)
    registry.disable("TEST_RULE")
    registry.enable("TEST_RULE")

    assert rule in registry.get_active_rules()


def test_registry_get_rules_by_stage(populated_registry):
    """Test getting rules by kill chain stage."""
    recon_rules = populated_registry.get_rules_by_stage("Reconnaissance")
    assert len(recon_rules) >= 1

    weap_rules = populated_registry.get_rules_by_stage("Weaponization")
    assert len(weap_rules) >= 3


def test_registry_statistics(populated_registry):
    """Test registry statistics."""
    stats = populated_registry.get_stats()

    assert stats["total_rules"] >= 17
    assert stats["active_rules"] >= 17
    assert stats["disabled_rules"] == 0
    assert "rules_by_stage" in stats


# ============================================================================
# KILL CHAIN STAGE 1: RECONNAISSANCE
# ============================================================================


@pytest.mark.asyncio
async def test_stage1_api_scanning_match():
    """Test Stage 1: API scanning detection matches."""
    rule = Stage1_APIScanning("STAGE_1", "Reconnaissance", "MEDIUM")

    event = {"type": "api_request", "actor_id": "attacker"}
    related = [event] + [
        {"type": "api_request", "actor_id": "attacker"}
        for _ in range(100)
    ]

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is True


@pytest.mark.asyncio
async def test_stage1_no_false_positive():
    """Test Stage 1: Normal API activity doesn't trigger."""
    rule = Stage1_APIScanning("STAGE_1", "Reconnaissance", "MEDIUM")

    event = {"type": "api_request", "actor_id": "normal_user"}
    related = [event]

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is False


@pytest.mark.asyncio
async def test_voting_velocity_spike_high_score():
    """Test VotingVelocitySpike rule with high anomaly score."""
    rule = VotingVelocitySpike("VELOCITY_SPIKE", "Reconnaissance", "HIGH")

    event = {"type": "governance_vote", "actor_id": "fast_voter"}
    anomaly_scores = {"voting_velocity": 0.9}

    matched = await rule.evaluate(event, [event], {}, anomaly_scores)
    assert matched is True


@pytest.mark.asyncio
async def test_voting_velocity_spike_low_score():
    """Test VotingVelocitySpike rule with low anomaly score."""
    rule = VotingVelocitySpike("VELOCITY_SPIKE", "Reconnaissance", "HIGH")

    event = {"type": "governance_vote", "actor_id": "normal_voter"}
    anomaly_scores = {"voting_velocity": 0.3}

    matched = await rule.evaluate(event, [event], {}, anomaly_scores)
    assert matched is False


# ============================================================================
# KILL CHAIN STAGE 2: WEAPONIZATION
# ============================================================================


@pytest.mark.asyncio
async def test_stage2_suspicious_proposal_match():
    """Test Stage 2: Suspicious proposal detection."""
    rule = Stage2_SuspiciousProposal("STAGE_2", "Weaponization", "HIGH")

    event = {
        "type": "proposal_created",
        "payload": {
            "proposal_value": 50000,
            "execution_delay_hours": 0.5,
        },
    }
    baselines = {"proposal_value_median": 1000}

    matched = await rule.evaluate(event, [], baselines, {})
    assert matched is True


@pytest.mark.asyncio
async def test_stage2_high_delay_safe():
    """Test Stage 2: High delay makes proposal safe."""
    rule = Stage2_SuspiciousProposal("STAGE_2", "Weaponization", "HIGH")

    event = {
        "type": "proposal_created",
        "payload": {
            "proposal_value": 50000,
            "execution_delay_hours": 48,
        },
    }
    baselines = {"proposal_value_median": 1000}

    matched = await rule.evaluate(event, [], baselines, {})
    assert matched is False


@pytest.mark.asyncio
async def test_new_actor_high_value():
    """Test NewActorHighValueProposal rule."""
    rule = NewActorHighValueProposal("NEW_ACTOR_HV", "Weaponization", "HIGH")

    event = {
        "type": "proposal_created",
        "actor_id": "new_actor",
        "payload": {"proposal_value": 10000},
    }
    baselines = {"proposal_value_median": 1000}
    related = [event]  # Only 1 event from this actor

    matched = await rule.evaluate(event, related, baselines, {})
    assert matched is True


@pytest.mark.asyncio
async def test_proposal_value_explosion():
    """Test ProposalValueExplosion rule (100x+ median)."""
    rule = ProposalValueExplosion("VALUE_EXPLOSION", "Weaponization", "CRITICAL")

    event = {
        "type": "proposal_created",
        "payload": {"proposal_value": 500000},  # 500x median
    }
    baselines = {"proposal_value_median": 1000}

    matched = await rule.evaluate(event, [], baselines, {})
    assert matched is True


@pytest.mark.asyncio
async def test_rapid_proposal_creation():
    """Test RapidProposalCreation rule."""
    rule = RapidProposalCreation("RAPID_PROPS", "Weaponization", "HIGH")

    actor_id = "spammer"
    event = {
        "type": "proposal_created",
        "actor_id": actor_id,
        "payload": {"proposal_value": 100},
    }

    # 15 proposals from same actor
    related = [event] + [
        {
            "type": "proposal_created",
            "actor_id": actor_id,
            "payload": {"proposal_value": 100},
        }
        for _ in range(14)
    ]

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is True


# ============================================================================
# KILL CHAIN STAGE 3: DELIVERY
# ============================================================================


@pytest.mark.asyncio
async def test_stage3_voting_bloc():
    """Test Stage 3: Large voting bloc detection."""
    rule = Stage3_LargeVotingBloc("STAGE_3", "Delivery", "HIGH")

    event = {
        "type": "governance_vote",
        "payload": {"proposal_id": "prop_1"},
    }

    related = [event] + [
        {
            "type": "governance_vote",
            "actor_id": f"voter_{i}",
            "payload": {"proposal_id": "prop_1"},
        }
        for i in range(60)
    ]

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is True


@pytest.mark.asyncio
async def test_multi_actor_coordination():
    """Test MultiActorCoordination rule (20+ actors)."""
    rule = MultiActorCoordination("MULTI_COORD", "Delivery", "HIGH")

    event = {
        "type": "governance_vote",
        "actor_id": "voter_0",
        "payload": {"proposal_id": "prop_1"},
    }

    # 25 different actors voting on same proposal
    related = [event] + [
        {
            "type": "governance_vote",
            "actor_id": f"voter_{i}",
            "payload": {"proposal_id": "prop_1"},
        }
        for i in range(25)
    ]

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is True


# ============================================================================
# KILL CHAIN STAGE 4: EXPLOITATION
# ============================================================================


@pytest.mark.asyncio
async def test_stage4_consensus_manipulation():
    """Test Stage 4: Consensus manipulation (suspiciously low variance)."""
    rule = Stage4_ConsensusManipulation("STAGE_4", "Exploitation", "CRITICAL")

    event = {
        "type": "judgment_created",
        "payload": {"consensus_variance": 0.01},
    }
    baselines = {"consensus_variance": 0.2}

    matched = await rule.evaluate(event, [], baselines, {})
    assert matched is True


@pytest.mark.asyncio
async def test_anomalous_consensus_too_perfect():
    """Test AnomalousConsensusVariance rule (too perfect)."""
    rule = AnomalousConsensusVariance("ANOM_CONSENSUS", "Exploitation", "HIGH")

    event = {
        "type": "judgment_created",
        "payload": {"consensus_variance": 0.02},
    }
    baselines = {"consensus_variance": 0.5}

    matched = await rule.evaluate(event, [], baselines, {})
    assert matched is True


@pytest.mark.asyncio
async def test_anomalous_consensus_too_chaotic():
    """Test AnomalousConsensusVariance rule (too chaotic)."""
    rule = AnomalousConsensusVariance("ANOM_CONSENSUS", "Exploitation", "HIGH")

    event = {
        "type": "judgment_created",
        "payload": {"consensus_variance": 3.0},  # 6x baseline
    }
    baselines = {"consensus_variance": 0.5}

    matched = await rule.evaluate(event, [], baselines, {})
    assert matched is True


# ============================================================================
# KILL CHAIN STAGE 6: COMMAND & CONTROL
# ============================================================================


@pytest.mark.asyncio
async def test_stage6_coordinated_voting():
    """Test Stage 6: Coordinated voting (80%+ proposals)."""
    rule = Stage6_CoordinatedVoting("STAGE_6", "C2", "CRITICAL")

    event = {
        "type": "governance_vote",
        "actor_id": "coordinator",
        "payload": {"proposal_id": "prop_1"},
    }

    # Coordinator votes on 9/10 proposals
    related = []
    for i in range(10):
        related.append({
            "type": "governance_vote",
            "actor_id": "coordinator" if i < 9 else "other",
            "payload": {"proposal_id": f"prop_{i}"},
        })

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is True


@pytest.mark.asyncio
async def test_bot_voting_pattern():
    """Test BotVotingPattern rule (identical voting)."""
    rule = BotVotingPattern("BOT_PATTERN", "C2", "HIGH")

    event = {
        "type": "governance_vote",
        "actor_id": "bot_1",
        "payload": {"proposal_id": "prop_1", "choice": "yes"},
    }

    # 10 votes, all "yes"
    related = [
        {
            "type": "governance_vote",
            "actor_id": "bot_1",
            "payload": {"proposal_id": f"prop_{i}", "choice": "yes"},
        }
        for i in range(10)
    ]

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is True


# ============================================================================
# KILL CHAIN STAGE 7: ACTIONS ON OBJECTIVES
# ============================================================================


@pytest.mark.asyncio
async def test_stage7_malicious_execution():
    """Test Stage 7: Malicious execution."""
    rule = Stage7_MaliciousExecution("STAGE_7", "Actions", "CRITICAL")

    event = {
        "type": "proposal_executed",
        "payload": {"proposal_id": "prop_1"},
    }

    related = [
        {
            "type": "proposal_created",
            "payload": {"proposal_id": "prop_1"},
        },
        event,
    ]

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is True


@pytest.mark.asyncio
async def test_governance_parameter_change():
    """Test GovernanceParameterChange rule."""
    rule = GovernanceParameterChange("PARAM_CHANGE", "Exploitation", "CRITICAL")

    event = {
        "type": "proposal_created",
        "payload": {
            "type": "governance",
            "description": "Change voting threshold",
        },
    }

    matched = await rule.evaluate(event, [], {}, {})
    assert matched is True


@pytest.mark.asyncio
async def test_treasury_address_change():
    """Test TreasuryAddressChange rule."""
    rule = TreasuryAddressChange("TREASURY_CHANGE", "Actions", "CRITICAL")

    event = {
        "type": "proposal_created",
        "payload": {
            "description": "Move treasury funds to new address",
        },
    }

    matched = await rule.evaluate(event, [], {}, {})
    assert matched is True


# ============================================================================
# RULE EXECUTOR TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_executor_execute_all_rules(executor):
    """Test rule executor evaluates all rules."""
    event = {
        "type": "governance_vote",
        "actor_id": "test_actor",
        "payload": {"proposal_id": "prop_1"},
    }

    results = await executor.execute(event, [event], {}, {})

    # Should have tried to evaluate all rules
    assert isinstance(results, list)


@pytest.mark.asyncio
async def test_executor_performance_stats(executor):
    """Test executor tracks performance statistics."""
    event = {"type": "governance_vote", "actor_id": "test_actor"}

    await executor.execute(event, [event], {}, {})
    stats = executor.get_rule_performance_stats()

    # Should have performance data
    assert isinstance(stats, dict)


# ============================================================================
# MISSING JUSTIFICATION TESTS
# ============================================================================


@pytest.mark.asyncio
async def test_missing_justification_no_text():
    """Test MissingJustification rule with no justification."""
    rule = MissingJustification("NO_JUST", "C2", "MEDIUM")

    actor_id = "suspicious_voter"
    event = {
        "type": "governance_vote",
        "actor_id": actor_id,
        "payload": {"proposal_id": "prop_1"},
    }

    # 10 votes from same actor, 8 without justification (80%)
    related = []
    for i in range(10):
        has_justification = i < 2
        related.append({
            "type": "governance_vote",
            "actor_id": actor_id,
            "payload": {
                "proposal_id": f"prop_{i}",
                "justification": "Valid reason" if has_justification else "",
            },
        })

    matched = await rule.evaluate(event, related, {}, {})
    assert matched is True


# ============================================================================
# EDGE CASES AND PERFORMANCE
# ============================================================================


@pytest.mark.asyncio
async def test_rule_match_count(executor):
    """Test rule tracks match count via executor."""
    rule = executor.registry._rules.get("STAGE_1_API_SCANNING")

    event = {"type": "api_request", "actor_id": "attacker"}
    related = [event] * 101

    initial_count = rule.match_count
    await executor.execute(event, related, {}, {})

    # Rule should have matched and incremented count
    assert rule.match_count > initial_count


@pytest.mark.asyncio
async def test_rule_with_missing_fields():
    """Test rule handles missing fields gracefully."""
    rule = Stage2_SuspiciousProposal("TEST", "Weaponization", "HIGH")

    event = {"type": "proposal_created"}  # Missing payload

    matched = await rule.evaluate(event, [], {}, {})
    # Should not crash, return False
    assert matched is False


@pytest.mark.asyncio
async def test_registry_with_duplicate_rule_ids():
    """Test registry handles duplicate rule IDs."""
    registry = RuleRegistry()
    rule1 = Stage1_APIScanning("SAME_ID", "Reconnaissance", "LOW")
    rule2 = Stage1_APIScanning("SAME_ID", "Reconnaissance", "LOW")

    registry.register(rule1)
    registry.register(rule2)  # Overwrites

    # Should have only 1 rule with that ID
    assert len([r for r in registry.get_active_rules() if r.rule_id == "SAME_ID"]) == 1


@pytest.mark.asyncio
async def test_executor_large_related_events(executor):
    """Test executor performance with large related event set."""
    event = {"type": "governance_vote", "actor_id": "test"}

    # 1000 related events
    related = [event] * 1000

    results = await executor.execute(event, related, {}, {})

    # Should complete without error
    assert isinstance(results, list)


def test_registry_statistics_accuracy(populated_registry):
    """Test registry statistics are accurate."""
    stats = populated_registry.get_stats()

    # Verify counts match
    assert stats["total_rules"] == len(populated_registry._rules)
    assert stats["active_rules"] + stats["disabled_rules"] == stats["total_rules"]

    # Disable a rule and verify
    populated_registry.disable("STAGE_1_API_SCANNING")
    updated_stats = populated_registry.get_stats()

    assert updated_stats["disabled_rules"] == 1
    assert updated_stats["active_rules"] == stats["active_rules"] - 1
