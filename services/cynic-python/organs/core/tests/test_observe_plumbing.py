"""
G1-G4 Plumbing regression tests.

G1 — task_result → /observe        : Ouroboros must emit after complete_task()
G2 — agent_id propagation          : observe() must carry the coordinated agent_id, not "unknown"
G3 — Soma gate events observed     : check_soma_gate() outcome must be observed to domain="soma"
G4 — KernelPort.observe() contract : KernelRestAdapter must implement the new port method
"""
import tempfile
import unittest
from typing import Optional, List, Dict, Tuple
from unittest import mock

from organs.core.adapters.kernel_rest import KernelRestAdapter, hermes_coord_agent_id
from organs.core.ouroboros import OuroborosLifecycle
from organs.core.ports.perception import KernelPort
from organs.core.ports.execution import ExecutorPort


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

class Response:
    def __init__(self, status_code: int, payload):
        self.status_code = status_code
        self._payload = payload

    def json(self):
        return self._payload


def make_adapter(tmp: str) -> KernelRestAdapter:
    return KernelRestAdapter(
        kernel_addr="http://kernel:3030",
        auth_key="test-key",
        organ_dir=tmp,
        domain_allowlist=set(),
        kind_allowlist=set(),
        domain_denylist=set(),
    )


class StubKernel(KernelPort):
    """In-memory stub that records all observe() calls."""

    def __init__(self, tasks=None, soma_decision="allocate"):
        self._tasks = tasks or []
        self._soma_decision = soma_decision
        self.observed: list[dict] = []
        self.completed: list[dict] = []

    def poll_tasks(self, limit=1) -> List[Dict]:
        batch, self._tasks = self._tasks[:limit], self._tasks[limit:]
        return batch

    def claim_task(self, task: dict) -> bool:
        return True

    def complete_task(self, task: dict, result: Optional[str], error: Optional[str]) -> bool:
        self.completed.append({"task": task, "result": result, "error": error})
        return True

    def release_task(self, task: dict, success: bool):
        pass

    def check_soma_gate(self, task_id: str) -> dict:
        if self._soma_decision == "queue":
            return {"decision": "queue", "data": {"wait_secs": 0}}
        return {"decision": "allocate", "data": {"slot_id": f"{task_id}-slot"}}

    def observe(self, domain: str, content: dict, agent_id: str) -> bool:
        self.observed.append({"domain": domain, "content": content, "agent_id": agent_id})
        return True

    def service_register(self, service_id, tier, node, capabilities, version=None, ttl_secs=120) -> bool:
        self.observed.append({"domain": "_service_register", "content": {
            "service_id": service_id, "tier": tier, "node": node,
            "capabilities": capabilities, "version": version,
        }, "agent_id": service_id})
        return True

    def service_heartbeat(self, service_id: str) -> bool:
        self.observed.append({"domain": "_service_heartbeat", "content": {"service_id": service_id}, "agent_id": service_id})
        return True

    def service_unregister(self, service_id: str) -> bool:
        self.observed.append({"domain": "_service_unregister", "content": {"service_id": service_id}, "agent_id": service_id})
        return True

    def coord_claim(self, task: dict) -> Tuple[str, List[str], Optional[str]]:
        agent_id = f"hermes-agent-{task.get('domain', 'x')}-{task.get('id', 'y')}"
        return agent_id[:64], [], None

    def coord_release(self, agent_id: str):
        pass


class StubExecutor(ExecutorPort):
    def __init__(self, result=None, error=None):
        self._result = result
        self._error = error

    def execute(self, task: dict, organ_dir: str, guard=None) -> Tuple[Optional[str], Optional[str]]:
        return self._result, self._error


def run_one_cycle(kernel: StubKernel, executor: StubExecutor):
    lifecycle = OuroborosLifecycle(kernel=kernel, executor=executor, organ_dir="/tmp", interval=0)
    lifecycle.running = True
    # Monkeypatch _sleep so the loop exits after processing all tasks
    original_sleep = lifecycle._sleep
    def _noop_sleep():
        lifecycle.running = False
    lifecycle._sleep = _noop_sleep
    lifecycle._loop()


# ---------------------------------------------------------------------------
# G4 — KernelPort.observe() contract implemented in KernelRestAdapter
# ---------------------------------------------------------------------------

class TestObserveAdapterG4(unittest.TestCase):

    def test_observe_posts_to_kernel_observe_endpoint(self):
        with tempfile.TemporaryDirectory() as tmp:
            a = make_adapter(tmp)
            with mock.patch(
                "organs.core.adapters.kernel_rest.requests.post",
                return_value=Response(200, {"ok": True}),
            ) as post:
                result = a.observe(
                    domain="forge",
                    content={"event": "test", "task_id": "t-1"},
                    agent_id="hermes-agent-forge-t-1",
                )
            self.assertTrue(result)
            url = post.call_args.args[0]
            self.assertEqual(url, "http://kernel:3030/observe")
            payload = post.call_args.kwargs["json"]
            self.assertEqual(payload["domain"], "forge")
            self.assertEqual(payload["agent_id"], "hermes-agent-forge-t-1")
            self.assertIn("event", payload["content"])

    def test_observe_returns_false_on_4xx(self):
        with tempfile.TemporaryDirectory() as tmp:
            a = make_adapter(tmp)
            with mock.patch(
                "organs.core.adapters.kernel_rest.requests.post",
                return_value=Response(503, {}),
            ):
                result = a.observe("soma", {}, "agent-x")
            self.assertFalse(result)

    def test_observe_returns_false_on_network_error(self):
        import requests as req_lib
        with tempfile.TemporaryDirectory() as tmp:
            a = make_adapter(tmp)
            with mock.patch(
                "organs.core.adapters.kernel_rest.requests.post",
                side_effect=req_lib.ConnectionError("unreachable"),
            ):
                result = a.observe("soma", {}, "agent-x")
            # fire-and-forget: must not raise
            self.assertFalse(result)


# ---------------------------------------------------------------------------
# G3 — Soma gate decision observed
# ---------------------------------------------------------------------------

class TestSomaObservedG3(unittest.TestCase):

    def test_allocate_gate_is_observed_to_soma_domain(self):
        task = {"id": "t-1", "domain": "token", "_source": "kernel",
                "objective": "test allocate"}
        kernel = StubKernel(tasks=[task], soma_decision="allocate")
        executor = StubExecutor(result="ok")
        run_one_cycle(kernel, executor)

        soma_events = [o for o in kernel.observed if o["domain"] == "soma"]
        self.assertEqual(len(soma_events), 1)
        self.assertEqual(soma_events[0]["content"]["event"], "gate_decision")
        self.assertEqual(soma_events[0]["content"]["decision"], "allocate")
        self.assertEqual(soma_events[0]["content"]["task_domain"], "token")

    def test_queue_gate_is_observed_to_soma_domain(self):
        task = {"id": "t-2", "domain": "token", "_source": "kernel",
                "objective": "test queue"}
        kernel = StubKernel(tasks=[task], soma_decision="queue")
        executor = StubExecutor(result="ok")
        run_one_cycle(kernel, executor)

        soma_events = [o for o in kernel.observed if o["domain"] == "soma"]
        self.assertGreater(len(soma_events), 0)
        self.assertEqual(soma_events[0]["content"]["decision"], "queue")


# ---------------------------------------------------------------------------
# G1 — task_result → /observe
# ---------------------------------------------------------------------------

class TestTaskResultObservedG1(unittest.TestCase):

    def test_task_completed_is_observed(self):
        task = {"id": "t-3", "domain": "forge", "_source": "kernel",
                "objective": "build the thing"}
        kernel = StubKernel(tasks=[task])
        executor = StubExecutor(result="built successfully")
        run_one_cycle(kernel, executor)

        forge_events = [o for o in kernel.observed
                        if o["domain"] == "forge" and o["content"].get("event") == "task_completed"]
        self.assertEqual(len(forge_events), 1)
        ev = forge_events[0]["content"]
        self.assertEqual(ev["task_id"], "t-3")
        self.assertIn("built", ev["result_preview"])

    def test_task_failed_is_observed(self):
        task = {"id": "t-4", "domain": "security", "_source": "kernel",
                "objective": "scan vulns"}
        kernel = StubKernel(tasks=[task])
        executor = StubExecutor(error="scan crashed")
        run_one_cycle(kernel, executor)

        sec_events = [o for o in kernel.observed
                      if o["domain"] == "security" and o["content"].get("event") == "task_failed"]
        self.assertEqual(len(sec_events), 1)
        self.assertIn("scan crashed", sec_events[0]["content"]["error"])


# ---------------------------------------------------------------------------
# G2 — agent_id propagation (no more "unknown")
# ---------------------------------------------------------------------------

class TestAgentIdPropagationG2(unittest.TestCase):

    def test_observe_carries_coordinated_agent_id_not_unknown(self):
        task = {"id": "task-99", "domain": "token", "_source": "kernel",
                "objective": "investigate rug"}
        kernel = StubKernel(tasks=[task])
        executor = StubExecutor(result="clean")
        run_one_cycle(kernel, executor)

        expected_agent_id = "hermes-agent-token-task-99"
        for obs in kernel.observed:
            self.assertEqual(obs["agent_id"], expected_agent_id,
                             f"observe() call had agent_id={obs['agent_id']!r}, "
                             f"expected {expected_agent_id!r}")


if __name__ == "__main__":
    unittest.main()
