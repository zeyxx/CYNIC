import json

from organs.twitter.src.entrypoints import organic_navigator


def test_ucb_selector_loads_history(tmp_path):
    log = tmp_path / "navigation_log.jsonl"
    log.write_text(
        json.dumps({"domain": "security", "reward": 0.8}) + "\n" +
        json.dumps({"domain": "security", "reward": 0.2}) + "\n"
    )

    selector = organic_navigator.UCB1Selector(log)

    assert selector.total_visits == 2
    assert selector.visit_counts["security"] == 2
    assert selector.reward_sums["security"] == 1.0


def test_load_targets_from_skill_adds_cashtag_targets(tmp_path):
    skill = tmp_path / "SKILL.md"
    skill.write_text("Watch $TALARIA and $METAO signals")

    targets = organic_navigator.load_targets_from_skill(skill)
    urls = {target["url"] for target in targets}

    assert "https://x.com/search?q=%24TALARIA&f=live" in urls
    assert "https://x.com/search?q=%24METAO&f=live" in urls


def test_apply_yield_boost_changes_weights(tmp_path):
    (tmp_path / "curation_yield.json").write_text(json.dumps({
        "domains": {
            "security": {"pct_of_total": 30},
            "dex": {"pct_of_total": 2},
        }
    }))
    targets = [
        {"url": "a", "domain": "security", "weight": 1.0},
        {"url": "b", "domain": "dex", "weight": 1.0},
    ]

    boosted = organic_navigator.apply_yield_boost(targets, tmp_path)
    assert boosted[0]["weight"] == 1.3
    assert boosted[1]["weight"] == 0.7


def test_get_cdp_endpoint_from_state_file(tmp_path):
    (tmp_path / "browser-state.json").write_text(json.dumps({"cdp_port": 40769}))

    assert organic_navigator.get_cdp_endpoint(tmp_path) == "http://127.0.0.1:40769"


def test_log_navigation_appends_jsonl(tmp_path):
    nav_log = tmp_path / "navigation_log.jsonl"
    organic_navigator.log_navigation(nav_log, {"domain": "security", "reward": 0.5})

    assert json.loads(nav_log.read_text()) == {"domain": "security", "reward": 0.5}
