"""Tests for outcome_collector_kernel — pure function tests only (no network)."""
import pytest
from outcome_collector_kernel import classify_outcome


class TestClassifyOutcome:
    def test_rug_price_and_holders(self):
        assert classify_outcome(-85.0, -60.0, -95.0) == "RUG"

    def test_rug_price_and_liquidity(self):
        assert classify_outcome(-82.0, -10.0, -92.0) == "RUG"

    def test_decline_on_price(self):
        assert classify_outcome(-40.0, 5.0, 0.0) == "DECLINE"

    def test_decline_on_holders(self):
        assert classify_outcome(-5.0, -25.0, 0.0) == "DECLINE"

    def test_stable(self):
        assert classify_outcome(-10.0, 5.0, 2.0) == "STABLE"

    def test_growth(self):
        assert classify_outcome(50.0, 15.0, 20.0) == "GROWTH"

    def test_boundary_rug_exact(self):
        assert classify_outcome(-80.0, -50.0, 0.0) == "RUG"

    def test_stable_near_zero(self):
        assert classify_outcome(0.0, 0.0, 0.0) == "STABLE"
