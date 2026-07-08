import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock
from io import StringIO

from organs.forge.src.core import cli_runtime
from organs.forge.src.core.organ_memory import OrganMemoryStore


class OrganMemoryStoreTests(unittest.TestCase):
    def test_status_does_not_create_local_store(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "state"
            report = OrganMemoryStore(root).status()

            self.assertEqual(report["status"], "ok")
            self.assertFalse(report["backend"]["sqlite_exists"])
            self.assertFalse(root.exists())

    def test_enqueue_outbox_writes_sqlite_and_journal(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "state"
            store = OrganMemoryStore(root)
            event = store.enqueue_outbox(
                event_type="agent_task_result_submit_failed",
                destination="kernel:/agent-tasks/result",
                payload={"task_id": "agent_tasks:abc", "success": False},
            )

            pending = store.list_pending_outbox()
            self.assertEqual(1, len(pending))
            self.assertEqual(event.idempotency_key, pending[0]["idempotency_key"])
            self.assertEqual("agent_tasks:abc", pending[0]["payload"]["task_id"])

            lines = (root / "journal" / "events.jsonl").read_text(encoding="utf-8").splitlines()
            self.assertEqual(1, len(lines))
            self.assertEqual(event.idempotency_key, json.loads(lines[0])["idempotency_key"])

            status = store.status()
            self.assertEqual(1, status["counts"]["outbox_pending"])
            self.assertEqual(1, status["counts"]["journal_events"])

    def test_outbox_is_idempotent_for_same_payload(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = OrganMemoryStore(Path(tmp) / "state")
            kwargs = {
                "event_type": "kernel_submit_failed",
                "destination": "kernel:/agent-tasks/result",
                "payload": {"task_id": "agent_tasks:abc"},
            }
            first = store.enqueue_outbox(**kwargs)
            second = store.enqueue_outbox(**kwargs)

            self.assertEqual(first.idempotency_key, second.idempotency_key)
            self.assertEqual(1, len(store.list_pending_outbox()))

    def test_flush_outbox_dry_run_does_not_mutate(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = OrganMemoryStore(Path(tmp) / "state")
            event = store.enqueue_outbox(
                event_type="compressed_observation",
                destination="kernel:/observe",
                payload={"status": "observed"},
            )

            report = store.flush_outbox(delivery=object(), dry_run=True)

            self.assertTrue(report["dry_run"])
            self.assertEqual(1, report["selected_count"])
            self.assertEqual(event.idempotency_key, report["events"][0]["idempotency_key"])
            self.assertEqual(1, store.status()["counts"]["outbox_pending"])

    def test_flush_outbox_marks_delivered_events_sent(self):
        class Delivery:
            def __init__(self):
                self.events = []

            def deliver(self, event):
                self.events.append(event)
                return {"status": "ok", "read_back": {"status": "observed"}}

        with tempfile.TemporaryDirectory() as tmp:
            store = OrganMemoryStore(Path(tmp) / "state")
            store.enqueue_outbox(
                event_type="compressed_observation",
                destination="kernel:/observe",
                payload={"status": "observed"},
            )

            report = store.flush_outbox(delivery=Delivery())

            self.assertEqual("ok", report["status"])
            self.assertEqual(1, report["delivered_count"])
            status = store.status()
            self.assertEqual(0, status["counts"]["outbox_pending"])
            self.assertEqual(1, status["counts"]["outbox_sent"])

    def test_flush_outbox_retries_then_deadletters_failed_events(self):
        class FailingDelivery:
            def deliver(self, event):
                return {"status": "failed", "error": "kernel down"}

        with tempfile.TemporaryDirectory() as tmp:
            store = OrganMemoryStore(Path(tmp) / "state")
            store.enqueue_outbox(
                event_type="compressed_observation",
                destination="kernel:/observe",
                payload={"status": "observed"},
            )

            first = store.flush_outbox(delivery=FailingDelivery(), max_attempts=2)
            second = store.flush_outbox(delivery=FailingDelivery(), max_attempts=2)

            self.assertEqual("degraded", first["status"])
            self.assertEqual(1, first["failed_count"])
            self.assertEqual(1, second["deadlettered_count"])
            status = store.status()
            self.assertEqual(0, status["counts"]["outbox_pending"])
            self.assertEqual(1, status["counts"]["deadletter"])

    def test_cli_flush_dry_run_emits_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            state = repo / "organs" / "forge" / "state"
            OrganMemoryStore(state).enqueue_outbox(
                event_type="compressed_observation",
                destination="kernel:/observe",
                payload={"status": "observed"},
            )

            with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
                status = cli_runtime.run_organ_memory_flush(
                    repo,
                    dry_run=True,
                    limit=10,
                    max_attempts=3,
                )

            payload = json.loads(stdout.getvalue())
            self.assertEqual(status, 0)
            self.assertTrue(payload["dry_run"])
            self.assertEqual(1, payload["selected_count"])


if __name__ == "__main__":
    unittest.main()
