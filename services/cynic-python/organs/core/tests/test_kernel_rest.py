import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from organs.core.adapters.kernel_rest import KernelRestAdapter, hermes_coord_agent_id


class Response:
    def __init__(self, status_code: int, payload: dict):
        self.status_code = status_code
        self.payload = payload

    def json(self):
        return self.payload


def adapter(organ_dir: str) -> KernelRestAdapter:
    return KernelRestAdapter(
        kernel_addr="http://kernel:3030",
        auth_key="test-key",
        organ_dir=organ_dir,
        domain_allowlist=set(),
        kind_allowlist=set(),
        domain_denylist=set(),
    )


class KernelRestClaimTests(unittest.TestCase):
    def test_kernel_claim_posts_agent_task_claim_before_local_lock(self):
        with tempfile.TemporaryDirectory() as tmp:
            task = {
                "_source": "kernel",
                "id": "agent_tasks:abc",
                "domain": "organ-forge",
                "kind": "hermes",
                "coord_targets": ["Makefile"],
            }

            with mock.patch(
                "organs.core.adapters.kernel_rest.requests.post",
                return_value=Response(200, {"claimed": True}),
            ) as post:
                claimed = adapter(tmp).claim_task(task)

            self.assertTrue(claimed)
            post.assert_called_once()
            self.assertEqual(
                post.call_args.args[0],
                "http://kernel:3030/agent-tasks/agent_tasks%3Aabc/claim",
            )
            self.assertEqual(
                post.call_args.kwargs["json"],
                {"agent_id": hermes_coord_agent_id(task)},
            )

            lock_file = Path(tmp) / "agent-tasks" / ".agent_tasks:abc.lock"
            self.assertTrue(lock_file.exists())
            lock = json.loads(lock_file.read_text())
            self.assertEqual(lock["source"], "kernel")
            self.assertEqual(lock["agent_id"], hermes_coord_agent_id(task))

    def test_kernel_claim_rejects_when_kernel_claim_returns_false(self):
        with tempfile.TemporaryDirectory() as tmp:
            task = {
                "_source": "kernel",
                "id": "agent_tasks:abc",
                "domain": "organ-forge",
                "kind": "hermes",
            }

            with mock.patch(
                "organs.core.adapters.kernel_rest.requests.post",
                return_value=Response(200, {"claimed": False}),
            ):
                claimed = adapter(tmp).claim_task(task)

            self.assertFalse(claimed)
            self.assertFalse((Path(tmp) / "agent-tasks" / ".agent_tasks:abc.lock").exists())


if __name__ == "__main__":
    unittest.main()
