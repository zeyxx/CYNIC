import json
from pathlib import Path

from organs.twitter.src.entrypoints import hermes_task_runner as runner


def write_task(root: Path, name: str, payload: dict) -> Path:
    tasks_dir = root / "agent-tasks"
    tasks_dir.mkdir(parents=True, exist_ok=True)
    task_file = tasks_dir / name
    task_file.write_text(json.dumps(payload))
    return task_file


def test_poll_tasks_reads_unlocked_json_tasks(tmp_path):
    write_task(tmp_path, "task-a.json", {"task_id": "task-a", "domain": "social-signal"})
    write_task(tmp_path, "task-b.json", {"id": "task-b", "domain": "other"})
    (tmp_path / "agent-tasks" / ".task-b.lock").write_text("{}")

    tasks = runner.poll_tasks(str(tmp_path))

    assert [task["id"] for task in tasks] == ["task-a"]
    assert tasks[0]["_file"].endswith("task-a.json")


def test_claim_task_is_exclusive(tmp_path):
    task_file = write_task(tmp_path, "task-a.json", {"task_id": "task-a"})
    task = {"id": "task-a", "_file": str(task_file)}

    assert runner.claim_task(task, str(tmp_path)) is True
    assert runner.claim_task(task, str(tmp_path)) is False
    assert (tmp_path / "agent-tasks" / ".task-a.lock").exists()


def test_release_task_success_deletes_task_and_lock(tmp_path):
    task_file = write_task(tmp_path, "task-a.json", {"task_id": "task-a"})
    task = {"id": "task-a", "_file": str(task_file)}
    assert runner.claim_task(task, str(tmp_path)) is True

    runner.release_task(task, str(tmp_path), success=True)

    assert not task_file.exists()
    assert not (tmp_path / "agent-tasks" / ".task-a.lock").exists()


def test_release_task_failure_keeps_task_and_removes_lock(tmp_path):
    task_file = write_task(tmp_path, "task-a.json", {"task_id": "task-a"})
    task = {"id": "task-a", "_file": str(task_file)}
    assert runner.claim_task(task, str(tmp_path)) is True

    runner.release_task(task, str(tmp_path), success=False)

    assert task_file.exists()
    assert not (tmp_path / "agent-tasks" / ".task-a.lock").exists()


def test_kernel_reporting_requires_explicit_endpoint(monkeypatch):
    monkeypatch.setattr(runner, "KERNEL_ADDR", "")

    assert runner.complete_task("task-a", result="ok") is False
