import json

from organs.twitter.src.entrypoints import gemini_learn_from_verdicts as learn


def test_load_observation_verdicts_normalizes_tweet_verdicts(tmp_path):
    (tmp_path / "verdicts").mkdir()
    (tmp_path / "dataset.jsonl").write_text(
        json.dumps({"tweet_id": "123", "narratives": ["rug_warning"]}) + "\n"
    )
    (tmp_path / "verdicts" / "123.json").write_text(
        json.dumps({"tweet_id": "123", "verdict": {"verdict": "HOWL", "domain": "D4"}})
    )

    verdicts = learn.load_observation_verdicts(str(tmp_path))

    assert verdicts == [
        {
            "observation_id": "tweet_123",
            "observation_domain": "D4",
            "observation_narratives": ["rug_warning"],
            "verdict": {"verdict": "HOWL", "domain": "D4"},
        }
    ]


def test_extract_patterns_builds_high_confidence_rule():
    patterns = learn.extract_patterns(
        [
            {
                "observation_domain": "D4",
                "observation_narratives": ["rug_warning"],
                "verdict": {"verdict": "HOWL"},
            },
            {
                "observation_domain": "D4",
                "observation_narratives": ["rug_warning"],
                "verdict": {"verdict": "WAG"},
            },
            {
                "observation_domain": "D4",
                "observation_narratives": ["rug_warning"],
                "verdict": {"verdict": "HOWL"},
            },
        ]
    )

    assert patterns["by_narrative"]["rug_warning"]["HOWL"] == 2
    assert patterns["by_domain"]["D4"]["WAG"] == 1
    assert patterns["high_confidence_rules"][0]["when"] == "narratives include 'rug_warning'"


def test_update_skill_md_replaces_existing_learned_patterns_section(tmp_path):
    skill_path = tmp_path / "SKILL.md"
    skill_path.write_text("# Skill\n\nKeep this.\n\n## Learned Patterns\n\nold data\n")
    patterns = learn.extract_patterns(
        [
            {
                "observation_domain": "D4",
                "observation_narratives": ["rug_warning"],
                "verdict": {"verdict": "HOWL"},
            },
            {
                "observation_domain": "D4",
                "observation_narratives": ["rug_warning"],
                "verdict": {"verdict": "WAG"},
            },
            {
                "observation_domain": "D4",
                "observation_narratives": ["rug_warning"],
                "verdict": {"verdict": "HOWL"},
            },
        ]
    )

    assert learn.update_skill_md(str(tmp_path), patterns) is True

    content = skill_path.read_text()
    assert "Keep this." in content
    assert "old data" not in content
    assert "rug_warning" in content

