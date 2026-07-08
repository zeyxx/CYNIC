from pathlib import Path

import pytest

from organs.twitter.src.entrypoints import hermes_x_login as login


def test_get_chrome_profile_uses_account_config(monkeypatch, tmp_path):
    profile = tmp_path / "profile"
    monkeypatch.setenv("HERMES_ACCOUNT", "personal")
    monkeypatch.setattr(
        login,
        "load_accounts_config",
        lambda: {"accounts": {"personal": {"profile": str(profile)}}},
    )

    assert login.get_chrome_profile() == profile


def test_get_chrome_profile_falls_back_to_default(monkeypatch):
    monkeypatch.delenv("HERMES_ACCOUNT", raising=False)

    assert login.get_chrome_profile() == login.DEFAULT_CHROME_PROFILE


def test_playwright_is_loaded_lazily(monkeypatch):
    real_import = __import__

    def fake_import(name, *args, **kwargs):
        if name == "playwright.sync_api":
            raise ImportError("missing playwright")
        return real_import(name, *args, **kwargs)

    monkeypatch.setattr("builtins.__import__", fake_import)

    with pytest.raises(RuntimeError, match="Playwright not installed"):
        login._load_playwright()

