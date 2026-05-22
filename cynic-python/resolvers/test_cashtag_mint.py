import pytest
from cashtag_mint import CashtagResolver

def test_known_symbol_resolves():
    """SOL should resolve to the well-known mint."""
    resolver = CashtagResolver(cache={
        "SOL": "So11111111111111111111111111111111"
    })
    assert resolver.resolve("SOL") == "So11111111111111111111111111111111"
    assert resolver.resolve("$SOL") == "So11111111111111111111111111111111"

def test_unknown_symbol_returns_none():
    resolver = CashtagResolver(cache={})
    assert resolver.resolve("NONEXISTENT_XYZ") is None

def test_ambiguous_symbol_returns_none():
    """Multiple mints for same symbol = ambiguous."""
    resolver = CashtagResolver(cache={
        "PEPE": "AMBIGUOUS"  # sentinel
    })
    assert resolver.resolve("PEPE") is None

def test_strip_dollar_sign():
    resolver = CashtagResolver(cache={"BTC": "mint123"})
    assert resolver.resolve("$BTC") == "mint123"
    assert resolver.resolve("BTC") == "mint123"
