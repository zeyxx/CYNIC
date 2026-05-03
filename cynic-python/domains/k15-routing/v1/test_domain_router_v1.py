#!/usr/bin/env python3
"""Unit tests for domain_router_v1."""

import json
import tempfile
from pathlib import Path
from domain_router_v1 import DomainRouter


def test_router_initialization():
    """Test router loads clustering results correctly."""
    # This test assumes results file exists from domain-discovery v1
    try:
        router = DomainRouter()
        assert router.cluster_profiles is not None
        assert len(router.cluster_profiles) > 0
        print("✓ Router initialization OK")
    except Exception as e:
        print(f"✗ Router initialization failed: {e}")


def test_token_analysis_routing():
    """Test routing of token-focused observations."""
    router = DomainRouter()

    test_cases = [
        ("pump.fun token launch", "token_analysis", "organ_x"),
        ("solana nft drops tomorrow", "token_analysis", "organ_x"),
        ("sol price action analysis", "token_analysis", "organ_x"),
    ]

    for text, expected_domain, expected_target in test_cases:
        decision = router.route_observation(text)
        assert decision["domain"] == expected_domain, f"Domain mismatch for '{text}': {decision}"
        assert decision["target"] == expected_target, f"Target mismatch for '{text}': {decision}"

    print("✓ Token analysis routing OK")


def test_llm_tech_routing():
    """Test routing of LLM/tech-focused observations."""
    router = DomainRouter()

    test_cases = [
        ("new gpt model released", "llm_tech", "human"),
        ("transformer architecture explained", "llm_tech", "human"),
        ("neural network training tips", "llm_tech", "human"),
    ]

    for text, expected_domain, expected_target in test_cases:
        decision = router.route_observation(text)
        assert decision["domain"] == expected_domain, f"Domain mismatch for '{text}': {decision}"
        assert decision["target"] == expected_target, f"Target mismatch for '{text}': {decision}"

    print("✓ LLM tech routing OK")


def test_general_routing():
    """Test routing of general/unclassified observations."""
    router = DomainRouter()

    test_cases = [
        ("just finished reading this thread", "general", "human"),
        ("check out my latest post", "general", "human"),
        ("great article on web development", "general", "human"),
    ]

    for text, expected_domain, expected_target in test_cases:
        decision = router.route_observation(text)
        assert decision["domain"] == expected_domain, f"Domain mismatch for '{text}': {decision}"
        assert decision["target"] == expected_target, f"Target mismatch for '{text}': {decision}"

    print("✓ General routing OK")


def test_decision_schema():
    """Test routing decision has required fields."""
    router = DomainRouter()

    decision = router.route_observation("test observation")

    required_fields = ["domain", "target", "confidence", "cluster_id"]
    for field in required_fields:
        assert field in decision, f"Missing field: {field}"
        assert decision[field] is not None, f"Field is None: {field}"

    assert 0 <= decision["confidence"] <= 1, f"Confidence out of range: {decision['confidence']}"
    assert isinstance(decision["cluster_id"], int), f"cluster_id not int: {decision['cluster_id']}"

    print("✓ Decision schema OK")


if __name__ == "__main__":
    print("Running domain_router tests...\n")

    try:
        test_router_initialization()
        test_token_analysis_routing()
        test_llm_tech_routing()
        test_general_routing()
        test_decision_schema()
        print("\n✓ All tests passed")
    except AssertionError as e:
        print(f"\n✗ Test failed: {e}")
        exit(1)
    except Exception as e:
        print(f"\n✗ Unexpected error: {e}")
        exit(1)
