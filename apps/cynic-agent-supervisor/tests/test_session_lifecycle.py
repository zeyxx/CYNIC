import importlib.util
import hashlib
import json
import sys
from types import SimpleNamespace
from threading import Event, Thread
import unittest
from pathlib import Path
from unittest.mock import patch


APP_DIR = Path(__file__).resolve().parents[1]


def load_plugin_package():
    spec = importlib.util.spec_from_file_location(
        "cynic_agent_supervisor_test",
        APP_DIR / "__init__.py",
        submodule_search_locations=[str(APP_DIR)],
    )
    if spec is None or spec.loader is None:
        raise AssertionError("plugin package could not be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class FakePluginContext:
    def __init__(self):
        self.hooks = {}
        self.tools = []

    def register_system_prompt_section(self, *args, **kwargs):
        pass

    def register_tool(self, **kwargs):
        self.tools.append(kwargs)

    def register_hook(self, name, callback):
        self.hooks[name] = callback


class SessionLifecycleTests(unittest.TestCase):
    def load_lifecycle_module(self):
        path = APP_DIR / "session_lifecycle.py"
        self.assertTrue(path.is_file(), "session lifecycle module is missing")
        spec = importlib.util.spec_from_file_location("cynic_session_lifecycle_test", path)
        if spec is None or spec.loader is None:
            raise AssertionError("session lifecycle module could not be loaded")
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        spec.loader.exec_module(module)
        return module

    def test_session_identity_is_stable_and_does_not_expose_source_id(self):
        module = self.load_lifecycle_module()
        agent_id = module.agent_id_for_session("opaque-hermes-session-id")
        self.assertEqual(agent_id, module.agent_id_for_session("opaque-hermes-session-id"))
        self.assertNotEqual(agent_id, module.agent_id_for_session("other-session"))
        self.assertNotIn("opaque-hermes-session-id", agent_id)

    def test_heartbeat_loop_renews_until_stopped(self):
        module = self.load_lifecycle_module()
        beats = []

        class StopAfterTwoWaits:
            def __init__(self):
                self.wait_count = 0

            def is_set(self):
                return False

            def wait(self, _interval):
                self.wait_count += 1
                return self.wait_count >= 2

        stop = StopAfterTwoWaits()
        module.heartbeat_loop(stop, lambda: beats.append("beat"), interval_seconds=300)
        self.assertEqual(beats, ["beat"])

    def test_session_heartbeat_persists_until_finalization(self):
        module = self.load_lifecycle_module()
        start_session = getattr(module.SessionLeaseLifecycle, "start_session", None)
        self.assertTrue(callable(start_session), "session-lifetime heartbeat is missing")
        if not callable(start_session):
            return
        agent_ids = []
        second_heartbeat = Event()
        released = []

        def heartbeat(agent_id):
            agent_ids.append(agent_id)
            if len(agent_ids) >= 2:
                second_heartbeat.set()

        lifecycle = module.SessionLeaseLifecycle(
            heartbeat=heartbeat,
            release=released.append,
            interval_seconds=0.01,
        )
        session_id = "session-for-lifecycle-test"
        expected_agent_id = module.agent_id_for_session(session_id)

        start_session(lifecycle, session_id)
        self.assertTrue(second_heartbeat.wait(1))
        self.assertEqual(agent_ids[:2], [expected_agent_id, expected_agent_id])
        self.assertEqual(released, [])
        self.assertTrue(lifecycle.finalize_session(session_id))
        self.assertTrue(lifecycle.finalize_session(session_id))
        self.assertEqual(released, [expected_agent_id])

    def test_finalization_does_not_release_until_in_flight_heartbeat_stops(self):
        module = self.load_lifecycle_module()
        heartbeat_started = Event()
        unblock_heartbeat = Event()
        heartbeat_finished = Event()
        released = []
        heartbeat_calls = 0

        def heartbeat(_agent_id):
            nonlocal heartbeat_calls
            heartbeat_calls += 1
            if heartbeat_calls == 2:
                heartbeat_started.set()
                unblock_heartbeat.wait()
                heartbeat_finished.set()

        lifecycle = module.SessionLeaseLifecycle(
            heartbeat=heartbeat,
            release=released.append,
            interval_seconds=0.01,
            join_timeout_seconds=0.01,
        )
        session_id = "session-with-in-flight-heartbeat"
        start_session = getattr(lifecycle, "start_session", None)
        self.assertTrue(callable(start_session), "session-lifetime heartbeat is missing")
        if not callable(start_session):
            return
        start_session(session_id)
        self.assertTrue(heartbeat_started.wait(1))
        self.assertFalse(lifecycle.finalize_session(session_id))
        self.assertEqual(released, [])

        unblock_heartbeat.set()
        self.assertTrue(heartbeat_finished.wait(1))
        self.assertTrue(lifecycle.finalize_session(session_id))
        self.assertEqual(len(released), 1)

    def test_finalization_waits_for_initial_heartbeat_before_releasing(self):
        module = self.load_lifecycle_module()
        heartbeat_started = Event()
        unblock_heartbeat = Event()
        heartbeat_finished = Event()
        released = []

        def heartbeat(_agent_id):
            heartbeat_started.set()
            unblock_heartbeat.wait()
            heartbeat_finished.set()

        lifecycle = module.SessionLeaseLifecycle(
            heartbeat=heartbeat,
            release=released.append,
            interval_seconds=60,
            join_timeout_seconds=0.01,
        )
        session_id = "session-with-in-flight-initial-heartbeat"
        starter = Thread(target=lifecycle.start_session, args=(session_id,))
        starter.start()
        self.assertTrue(heartbeat_started.wait(1))

        try:
            self.assertFalse(lifecycle.finalize_session(session_id))
            self.assertEqual(released, [])
        finally:
            unblock_heartbeat.set()
        starter.join(timeout=1)
        self.assertTrue(heartbeat_finished.wait(1))
        self.assertTrue(lifecycle.finalize_session(session_id))
        self.assertEqual(len(released), 1)

    def test_plugin_registers_session_heartbeat_and_final_session_cleanup(self):
        plugin = load_plugin_package()
        context = FakePluginContext()
        plugin.register(context)
        required = {
            "on_session_start",
            "pre_llm_call",
            "on_session_finalize",
            "on_session_reset",
        }
        self.assertTrue(required.issubset(context.hooks), f"missing hooks: {required - context.hooks.keys()}")
        self.assertNotIn("on_session_end", context.hooks)

    def test_session_prompt_exposes_only_the_derived_agent_identity(self):
        plugin = load_plugin_package()
        session_id = "opaque-hermes-session-id"
        expected = "hermes:session:" + hashlib.sha256(session_id.encode()).hexdigest()[:24]
        with patch.object(plugin.shutil, "which", return_value=None):
            prompt = plugin._cynic_supervisor_context({"session_id": session_id})
        self.assertIn(expected, prompt)
        self.assertNotIn(session_id, prompt)
        self.assertIn("--ttl-seconds 1800", prompt)
        self.assertIn("every 300 seconds", prompt)
        self.assertIn("while the session is active", prompt)

    def test_prompt_filters_claim_status_to_the_current_session(self):
        plugin = load_plugin_package()
        session_id = "session-for-status-filter"
        expected = "hermes:session:" + hashlib.sha256(session_id.encode()).hexdigest()[:24]
        commands = []

        def run_command(command, **_kwargs):
            commands.append(command)
            if command[1:4] == ["agent", "session", "context"]:
                return SimpleNamespace(
                    returncode=0,
                    stdout='{"builder":"unknown","git_sha":"abc","version":"test"}',
                )
            if command[1:3] == ["agent", "status"]:
                return SimpleNamespace(
                    returncode=0,
                    stdout='{"active_claims":[],"total_claims":0}',
                )
            raise AssertionError(f"unexpected command: {command}")

        with patch.object(plugin.shutil, "which", return_value="/usr/bin/cynic"), patch.object(
            plugin.subprocess, "run", side_effect=run_command
        ), patch.object(plugin, "AGENT_SPECS", {}):
            plugin._cynic_supervisor_context({"session_id": session_id})

        status_commands = [command for command in commands if command[1:3] == ["agent", "status"]]
        self.assertEqual(len(status_commands), 1)
        self.assertIn("--agent-id", status_commands[0])
        self.assertIn(expected, status_commands[0])

    def test_heartbeat_refreshes_each_claim_owned_by_the_session(self):
        plugin = load_plugin_package()
        heartbeat = getattr(plugin, "_heartbeat_session_claims", None)
        self.assertTrue(callable(heartbeat), "session heartbeat adapter is missing")
        if not callable(heartbeat):
            return
        agent_id = "hermes:session:test-session"
        claims = [
            {"claim_id": "lease-a", "target": "crates/alpha"},
            {"claim_id": "lease-b", "target": "crates/beta"},
        ]
        calls = []

        def run_cynic(args, **_kwargs):
            calls.append(args)
            if args[:2] == ["agent", "status"]:
                return {"active_claims": claims}
            if args[:2] == ["agent", "heartbeat"]:
                return {"status": "heartbeat"}
            raise AssertionError(f"unexpected command: {args}")

        with patch.object(plugin, "_find_cynic_repo_root", return_value="/cynic"), patch.object(
            plugin, "_run_cynic_command", side_effect=run_cynic
        ):
            heartbeat(agent_id)

        heartbeats = [args for args in calls if args[:2] == ["agent", "heartbeat"]]
        self.assertEqual(len(heartbeats), 2)
        self.assertTrue(all("--agent-id" in args and agent_id in args for args in heartbeats))
        self.assertTrue(all("--lease-id" in args for args in heartbeats))

    def test_session_close_releases_claims_and_reads_back_receipt(self):
        plugin = load_plugin_package()
        release_session = getattr(plugin, "_release_session_claims", None)
        self.assertTrue(callable(release_session), "session close adapter is missing")
        if not callable(release_session):
            return
        agent_id = "hermes:session:test-session"
        claim = {"claim_id": "lease-a", "target": "crates/alpha"}
        status_count = 0
        calls = []
        receipt = {}

        def run_cynic(args, **_kwargs):
            nonlocal status_count
            calls.append(args)
            if args[:2] == ["agent", "status"]:
                status_count += 1
                return {"active_claims": [claim] if status_count == 1 else []}
            if args[:2] == ["agent", "release"]:
                return {"status": "released", "claim_id": "lease-a"}
            if args[:2] == ["coordination", "append"]:
                body = args[args.index("--body") + 1]
                receipt.update({"agent": args[args.index("--agent") + 1], "kind": "session_close", "body": body})
                return dict(receipt)
            if args[:2] == ["coordination", "read"]:
                return {"entries": [dict(receipt)]}
            raise AssertionError(f"unexpected command: {args}")

        with patch.object(plugin, "_find_cynic_repo_root", return_value="/cynic"), patch.object(
            plugin, "_run_cynic_command", side_effect=run_cynic
        ):
            release_session(agent_id)

        release_calls = [args for args in calls if args[:2] == ["agent", "release"]]
        self.assertEqual(len(release_calls), 1)
        self.assertIn("--target", release_calls[0])
        body = json.loads(receipt["body"])
        self.assertEqual(body["session_id"], agent_id)
        self.assertEqual(body["repo_root"], "/cynic")
        self.assertTrue(body["claims_released"])
        self.assertEqual(body["worktree_fate"], "retained")
        self.assertTrue(body["proof_ref"].startswith("sha256:"))

    def test_session_start_records_identity_mapping_and_reads_it_back(self):
        plugin = load_plugin_package()
        record_start = getattr(plugin, "_record_session_start", None)
        self.assertTrue(callable(record_start), "session identity receipt is missing")
        if not callable(record_start):
            return
        agent_id = "hermes:session:test-session"
        calls = []
        receipt = {}
        reads = []

        def run_cynic(args, **_kwargs):
            calls.append(args)
            if args[:2] == ["coordination", "append"]:
                body = args[args.index("--body") + 1]
                receipt.update({"agent": agent_id, "kind": "session_open", "body": body})
                return dict(receipt)
            if args[:2] == ["coordination", "read"]:
                reads.append(True)
                if len(reads) == 1:
                    return {"entries": []}
                return {"entries": [dict(receipt)]}
            raise AssertionError(f"unexpected command: {args}")

        with patch.object(plugin, "_find_cynic_repo_root", return_value="/cynic"), patch.object(
            plugin, "_run_cynic_command", side_effect=run_cynic
        ):
            record_start(agent_id)
            record_start(agent_id)

        body = json.loads(receipt["body"])
        self.assertEqual(body["session_id"], agent_id)
        self.assertEqual(body["agent_id"], agent_id)
        self.assertEqual(body["repo_root"], "/cynic")
        self.assertEqual(sum(args[:2] == ["coordination", "append"] for args in calls), 1)
        self.assertEqual(len(reads), 2)

    def test_supervisor_decodes_bytes_from_timed_out_subprocesses(self):
        load_plugin_package()
        supervisor = sys.modules["cynic_agent_supervisor_test.supervisor"]
        output_text = getattr(supervisor, "_output_text", None)
        self.assertTrue(callable(output_text), "subprocess output normalizer is missing")
        if not callable(output_text):
            return
        self.assertEqual(output_text(b"partial output"), "partial output")
        self.assertEqual(output_text(b"\xff"), "\ufffd")
        self.assertEqual(output_text(None), "")

    def test_supervise_handler_binds_claim_and_release_to_session_identity(self):
        load_plugin_package()
        tools = sys.modules["cynic_agent_supervisor_test.tools"]
        session_id = "opaque-tool-session-id"
        expected_agent_id = "hermes:session:" + hashlib.sha256(session_id.encode()).hexdigest()[:24]
        observation = SimpleNamespace(cli=SimpleNamespace(available=True), agent="codex")
        claim = SimpleNamespace(claimed=True, agent_id=expected_agent_id)
        execution = SimpleNamespace(timed_out=False, exit_code=0)

        with patch.object(tools, "observe", return_value=observation), patch.object(
            tools, "reconcile_claim", return_value=claim
        ) as reconcile, patch.object(tools, "execute_agent_cli", return_value=execution), patch.object(
            tools, "release_claim", return_value=SimpleNamespace(status="released")
        ) as release, patch.object(tools, "build_proof", return_value=object()), patch.object(
            tools, "proof_to_json", return_value="proof"
        ):
            result = tools.handle_cynic_agent_supervise(
                {"agent": "codex", "prompt": "bounded", "working_dir": "/tmp"},
                session_id=session_id,
            )

        self.assertEqual(result, "proof")
        reconcile.assert_called_once_with(observation, None, agent_id=expected_agent_id)
        release.assert_called_once_with(observation, claim, execution, agent_id=expected_agent_id)

    def test_claim_and_release_commands_keep_the_explicit_session_identity(self):
        load_plugin_package()
        supervisor = sys.modules["cynic_agent_supervisor_test.supervisor"]
        agent_id = "hermes:session:0123456789abcdef01234567"
        observation = SimpleNamespace(working_dir="/cynic", agent="codex")
        execution = SimpleNamespace(exit_code=0, duration_seconds=0.1)
        calls = []

        def run_cynic(args, **_kwargs):
            calls.append(args)
            if args[:2] == ["agent", "claim"]:
                return {"status": "claimed", "lease_id": "lease-a", "ttl_seconds": 1800}
            if args[:2] == ["agent", "release"]:
                return {"status": "released"}
            raise AssertionError(f"unexpected command: {args}")

        with patch.object(supervisor, "_find_cynic_repo_root", return_value="/cynic"), patch.object(
            supervisor, "_run_cynic_command", side_effect=run_cynic
        ):
            claim = supervisor.reconcile_claim(observation, "crates/alpha", agent_id=agent_id)
            release = supervisor.release_claim(observation, claim, execution)

        self.assertEqual(claim.agent_id, agent_id)
        self.assertEqual(release.status, "released")
        for command in calls:
            self.assertIn("--agent-id", command)
            self.assertEqual(command[command.index("--agent-id") + 1], agent_id)


if __name__ == "__main__":
    unittest.main()
