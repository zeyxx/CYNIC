from organs.forge.src.core.anvil import (
    BranchCounts,
    DirtyCounts,
    RepoAnvilService,
    RepoAnvilSnapshot,
)


def test_anvil_perception_payload_is_deterministic():
    snapshot = RepoAnvilSnapshot(
        repo="/repo",
        branch="main",
        head="abc123",
        dirty=DirtyCounts(total=10, modified=4, untracked=3, deleted=2, added=1, renamed=0),
        branches=BranchCounts(local=7, remote=2),
        gates={"gate-0": 100, "gate-1": 0, "gate-2": 0, "gate-passed": 0},
        timestamp="2026-06-30T00:00:00Z",
    )

    payload = RepoAnvilService().perception_payload(snapshot, now=100)

    assert payload["repo"] == "/repo"
    assert payload["dirty"]["total"] == 10
    assert payload["branches"]["local"] == 7
    assert payload["health_score"] == 0.79


def test_anvil_audit_entry_is_json_ready():
    entry = RepoAnvilService().audit_entry(
        timestamp="2026-06-30T00:00:00Z",
        action="probe",
        payload={"ok": True},
        result="success",
    )

    assert entry == {
        "timestamp": "2026-06-30T00:00:00Z",
        "action": "probe",
        "payload": {"ok": True},
        "result": "success",
    }
