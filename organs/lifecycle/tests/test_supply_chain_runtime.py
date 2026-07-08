from __future__ import annotations

import asyncio
import json
import tempfile
from pathlib import Path
import unittest

from organs.lifecycle.src.adapters.registry_supply_chain_adapter import RegistrySupplyChainAdapter
from organs.lifecycle.src.adapters.hermes_sovereign_inference_adapter import HermesAgentSovereignInferenceAdapter
from organs.lifecycle.src.core.dependency_convergence import (
    build_dependency_convergence_hermes_task,
    build_dependency_convergence_pack,
)
from organs.lifecycle.src.entrypoints.controller import LifecycleController, LifecycleControllerPaths
from organs.lifecycle.src.entrypoints import watchdog as watchdog_entrypoint
from organs.lifecycle.src.core.runtime import LifecycleRuntime


def _registry_fixture() -> dict:
    return {
        "nodes": [
            {
                "name": "node-a",
                "role": "role-a",
                "depends_on": ["organ-b"],
            },
            {
                "name": "node-c",
                "role": "role-c",
                "depends_on": ["missing-service"],
            },
            {
                "name": "gpu-host",
                "role": "inference_node",
                "constellation": "inference",
            },
        ],
        "organs": [
            {
                "name": "organ-b",
                "depends_on": ["node-a"],
            }
        ],
        "service_placement": {
            "cynic-kernel": {"role": "kernel_primary"},
        },
        "lifecycle": {
            "kernel_release": {
                "orchestrator_organ": "organ-b",
                "task_kind": "hermes",
                "task_domain": "organ-lifecycle",
                "build_runner_role": "role-a",
                "deploy_target_role": "role-c",
                "deploy_target_flag": "deploy_target",
                "required_node_status": "active",
                "activation_requirements": {
                    "role-a": {
                        "network": ["synapse"],
                        "filesystem": ["repo_root"],
                        "services": ["cynic-kernel"],
                    },
                    "role-c": {
                        "network": ["synapse"],
                        "filesystem": ["repo_root"],
                        "services": ["cynic-kernel"],
                    },
                },
                "fleet_requirements": {
                    "provider": "synapse",
                    "required": False,
                    "require_online": False,
                },
                "kernel_bus_probe": {
                    "path": "/health",
                    "accepted_status": [200, 503],
                    "requires_bearer_auth": True,
                    "timeout_seconds": 10,
                },
                "runtime_paths": {
                    "repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}",
                },
                "runtime_path_proofs": {
                    "repo_root": {
                        "kind": "git_worktree",
                        "marker": ".git",
                        "sentinels": ["AGENTS.md", "infra/registry.json"],
                    }
                },
                "activation_evidence_contract": {
                    "patch_field": "activation_evidence_patch",
                    "node_field": "node",
                    "runtime_paths_field": "activation_evidence.runtime_paths",
                    "services_field": "activation_evidence.services",
                    "verified_requires": ["marker_exists", "sentinels_verified"],
                    "service_active_requires": ["declared", "active"],
                    "ingestion_mode": "validated_merge",
                },
                "repo_targets": ["infra/registry.json"],
                "request_profiles": {"gate-2": ["build-and-test"]},
                "intents": {
                    "build-and-test": {
                        "objective": "run gate-2 validation on registry build runner",
                        "deploy": False,
                    }
                },
                "workflow": {
                    "name": "kernel_release",
                    "owner": "organ-forge",
                    "state_layers": {
                        "desired": {"source": "human_intent"},
                        "declared": {"source": "infra/registry.json"},
                        "observed": {"source": "runtime_probes"},
                        "effective": {"source": "organ-forge"},
                    },
                    "canonical_stages": [
                        {
                            "name": "local_validation",
                            "scope": "cortex-local",
                            "make_target": "check",
                            "allowed_when_lifecycle_planned": True,
                        },
                        {
                            "name": "gate_2_request",
                            "scope": "organ-forge",
                            "make_target": "gate-2-request",
                            "request_profile": "gate-2",
                            "allowed_when_lifecycle_planned": False,
                        },
                    ],
                    "planned_policy": {
                        "blocks_workflow_critical_edits": True,
                        "critical_targets": ["infra/registry.json"],
                    },
                },
            }
        },
    }


class FakeObservationPort:
    def __init__(self):
        self.events: list[dict] = []

    async def observe(self, event):
        self.events.append(event)
        return {"status": "observed"}

    def audit_query(self, query):
        return {"query": query}


class FakeAnalyzer:
    def __init__(self, report: dict):
        self.report = report

    def analyze_supply_chain(self):
        return self.report


class FakeRuntime:
    def __init__(self, report: dict):
        self.report = report

    async def run_once(self):
        return {
            "status": self.report.get("status", "unknown"),
            "report": self.report,
            "signal": {"timestamp": "2026-06-29T20:00:00Z"},
        }


class SupplyChainAnalysisTests(unittest.TestCase):
    def test_analysis_resolves_node_declared_services(self):
        registry = {
            "nodes": [
                {
                    "name": "node-a",
                    "depends_on": ["llama-embed"],
                    "services": [{"name": "llama-embed"}],
                }
            ],
            "lifecycle": {"kernel_release": {}},
        }

        report = RegistrySupplyChainAdapter(registry_data=registry).analyze_supply_chain()

        self.assertEqual(report["status"], "ok")
        self.assertEqual(report["summary"]["unresolved_edges"], 0)
        self.assertEqual(report["summary"]["cycle_count"], 0)

    def test_analysis_reports_cycles_and_unresolved_dependencies(self):
        report = RegistrySupplyChainAdapter(registry_data=_registry_fixture()).analyze_supply_chain()

        self.assertEqual(report["status"], "blocked")
        self.assertEqual(report["summary"]["cycle_count"], 1)
        self.assertEqual(report["summary"]["unresolved_edges"], 1)
        self.assertTrue(
            any(set(cycle) >= {"node-a", "organ-b"} for cycle in report["cycles"]),
        )
        self.assertTrue(
            any(edge["target"] == "missing-service" and not edge["resolved"] for edge in report["unresolved_dependencies"]),
        )

    def test_dependency_convergence_pack_requests_sovereign_inference_for_cycles(self):
        registry = _registry_fixture()
        report = RegistrySupplyChainAdapter(registry_data=registry).analyze_supply_chain()

        pack = build_dependency_convergence_pack(report, registry)

        self.assertEqual(pack["schema"], "cynic.dependency_convergence_pack.v1")
        self.assertEqual(pack["status"], "blocked")
        self.assertEqual(
            pack["inference_request"]["schema"],
            "cynic.dependency_convergence_inference_request.v1",
        )
        self.assertEqual(pack["inference_request"]["sovereign_inference"]["node"], "gpu-host")
        self.assertEqual(pack["proposal"]["schema"], "cynic.dependency_convergence_proposal.v1")
        self.assertTrue(pack["proposal"]["requires_sovereign_inference"])
        self.assertEqual(pack["proposal"]["targets"], ["infra/registry.json"])
        self.assertEqual(pack["dispatch"]["consumer"], "organ-lifecycle-hermes-agent")
        self.assertEqual(
            pack["operator_interface_proposal"]["schema"],
            "cynic.agent_proposal.v1",
        )
        self.assertEqual(pack["operator_interface_proposal"]["visibility"], "operator_private")
        self.assertEqual(pack["operator_interface_proposal"]["operator_review"]["state"], "pending")
        self.assertFalse(pack["operator_interface_proposal"]["public_projection"]["eligible"])
        self.assertEqual(
            pack["operator_interface_proposal"]["public_projection"]["redaction_status"],
            "needs_review",
        )

    def test_dependency_convergence_hermes_task_is_proposal_only(self):
        registry = _registry_fixture()
        report = RegistrySupplyChainAdapter(registry_data=registry).analyze_supply_chain()
        pack = build_dependency_convergence_pack(report, registry)

        task = build_dependency_convergence_hermes_task(pack)

        self.assertEqual(task["kind"], "hermes")
        self.assertEqual(task["domain"], "organ-lifecycle")
        self.assertEqual(task["consumer"], "organ-lifecycle-hermes-agent")
        content = task["content"]
        self.assertEqual(content["schema"], "cynic.dependency_convergence_hermes_task.v1")
        self.assertEqual(content["mode"], "proposal_only")
        self.assertFalse(content["mutation_allowed"])
        self.assertEqual(content["targets"], ["infra/registry.json"])
        self.assertEqual(content["coord_targets"], ["infra/registry.json"])
        self.assertEqual(
            content["output_contract"]["schema"],
            "cynic.dependency_convergence_proposal.v1",
        )
        self.assertEqual(
            content["operator_interface_proposal"]["schema"],
            "cynic.agent_proposal.v1",
        )
        self.assertEqual(
            content["operator_interface_proposal"]["public_projection"]["eligible"],
            False,
        )

    def test_hermes_sovereign_inference_adapter_dispatches_kernel_task(self):
        registry = _registry_fixture()
        report = RegistrySupplyChainAdapter(registry_data=registry).analyze_supply_chain()
        pack = build_dependency_convergence_pack(report, registry)
        dispatched = []

        class FakeDispatcher:
            def dispatch_task(self, payload):
                dispatched.append(payload)
                return True

        result = HermesAgentSovereignInferenceAdapter(FakeDispatcher()).request_dependency_convergence(pack)

        self.assertEqual(result["status"], "queued")
        self.assertEqual(result["adapter"], "hermes-agent")
        self.assertEqual(result["consumer"], "organ-lifecycle-hermes-agent")
        self.assertEqual(dispatched[0]["kind"], "hermes")
        self.assertEqual(dispatched[0]["domain"], "organ-lifecycle")
        content = json.loads(dispatched[0]["content"])
        self.assertFalse(content["mutation_allowed"])
        self.assertEqual(content["targets"], ["infra/registry.json"])
        self.assertEqual(content["operator_interface_proposal"]["visibility"], "operator_private")

    def test_hermes_sovereign_inference_adapter_skips_when_no_inference_required(self):
        registry = {"nodes": [], "lifecycle": {"kernel_release": {}}}
        report = RegistrySupplyChainAdapter(registry_data=registry).analyze_supply_chain()
        pack = build_dependency_convergence_pack(report, registry)

        class FailingDispatcher:
            def dispatch_task(self, payload):
                raise AssertionError("should not dispatch")

        result = HermesAgentSovereignInferenceAdapter(FailingDispatcher()).request_dependency_convergence(pack)

        self.assertEqual(result["status"], "skipped")
        self.assertEqual(result["reason"], "deterministic analysis found no cycle requiring sovereign inference")

    def test_runtime_emits_observation_for_supply_chain_report(self):
        report = RegistrySupplyChainAdapter(registry_data=_registry_fixture()).analyze_supply_chain()
        fake_observation = FakeObservationPort()
        runtime = LifecycleRuntime(analyzer=FakeAnalyzer(report), observation_port=fake_observation)

        result = asyncio.run(runtime.run_once())

        self.assertEqual(result["status"], "blocked")
        self.assertEqual(len(fake_observation.events), 1)
        self.assertEqual(fake_observation.events[0]["tool"], "lifecycle_runtime")
        self.assertEqual(fake_observation.events[0]["status"], "blocked")
        self.assertEqual(fake_observation.events[0]["consumer"], "organ-forge-hermes-agent")

    def test_controller_updates_ssot_from_desired_state(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo_root = Path(tmpdir)
            lifecycle_root = repo_root / "organs" / "lifecycle"
            desired_path = lifecycle_root / "desired" / "state.json"
            ssot_path = lifecycle_root / "state" / "SSOT.json"
            desired_path.parent.mkdir(parents=True, exist_ok=True)
            ssot_path.parent.mkdir(parents=True, exist_ok=True)
            desired_path.write_text(json.dumps({"desired": {"auto_remediation_enabled": True}}))
            ssot_path.write_text(json.dumps({"status": "stale"}))

            report = RegistrySupplyChainAdapter(registry_data=_registry_fixture()).analyze_supply_chain()
            controller = LifecycleController(
                runtime=FakeRuntime(report),
                paths=LifecycleControllerPaths(
                    desired_path=desired_path,
                    ssot_path=ssot_path,
                ),
            )

            result = asyncio.run(controller.run_once())

            self.assertEqual(result["status"], "blocked")
            persisted = json.loads(ssot_path.read_text())
            self.assertEqual(persisted["last_synced"], "2026-06-29T20:00:00Z")
            self.assertEqual(persisted["status"], "degraded")
            self.assertEqual(persisted["active_provisioning_tasks"], 1)

    def test_controller_health_reports_fresh_active_ssot(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo_root = Path(tmpdir)
            lifecycle_root = repo_root / "organs" / "lifecycle"
            desired_path = lifecycle_root / "desired" / "state.json"
            ssot_path = lifecycle_root / "state" / "SSOT.json"
            desired_path.parent.mkdir(parents=True, exist_ok=True)
            ssot_path.parent.mkdir(parents=True, exist_ok=True)
            desired_path.write_text(json.dumps({"desired": {"auto_remediation_enabled": True}}))
            ssot_path.write_text(json.dumps({"status": "active", "last_synced": "2026-06-29T20:00:00Z"}))

            controller = LifecycleController(
                paths=LifecycleControllerPaths(
                    desired_path=desired_path,
                    ssot_path=ssot_path,
                ),
            )

            health = controller.health()

            self.assertEqual(health["status"], "ok")
            self.assertEqual(health["reason"], "fresh_active_ssot")
            self.assertTrue(health["fresh"])
            self.assertIsNotNone(health["ssot_age_seconds"])

    def test_controller_health_blocks_when_ssot_missing(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo_root = Path(tmpdir)
            lifecycle_root = repo_root / "organs" / "lifecycle"
            desired_path = lifecycle_root / "desired" / "state.json"
            ssot_path = lifecycle_root / "state" / "SSOT.json"
            desired_path.parent.mkdir(parents=True, exist_ok=True)
            desired_path.write_text(json.dumps({"desired": {"auto_remediation_enabled": True}}))

            controller = LifecycleController(
                paths=LifecycleControllerPaths(
                    desired_path=desired_path,
                    ssot_path=ssot_path,
                ),
            )

            health = controller.health()

            self.assertEqual(health["status"], "blocked")
            self.assertEqual(health["reason"], "ssot_missing")
            self.assertFalse(health["fresh"])

    def test_controller_reconcile_noops_when_health_ok(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo_root = Path(tmpdir)
            lifecycle_root = repo_root / "organs" / "lifecycle"
            desired_path = lifecycle_root / "desired" / "state.json"
            ssot_path = lifecycle_root / "state" / "SSOT.json"
            desired_path.parent.mkdir(parents=True, exist_ok=True)
            ssot_path.parent.mkdir(parents=True, exist_ok=True)
            desired_path.write_text(json.dumps({"desired": {"auto_remediation_enabled": True}}))
            ssot_path.write_text(json.dumps({"status": "active", "last_synced": "2026-06-29T20:00:00Z"}))

            controller = LifecycleController(
                paths=LifecycleControllerPaths(
                    desired_path=desired_path,
                    ssot_path=ssot_path,
                ),
            )

            result = asyncio.run(controller.reconcile())

            self.assertEqual(result["action"], "noop")
            self.assertEqual(result["health_after"]["status"], "ok")

    def test_watchdog_reconciles_stale_state(self):
        class FakeController:
            def __init__(self):
                self.calls = []

            async def reconcile(self):
                self.calls.append("reconcile")
                return {
                    "action": "reconciled",
                    "health_before": {"status": "degraded"},
                    "health_after": {"status": "ok"},
                }

        result = asyncio.run(watchdog_entrypoint.run_watchdog_once(FakeController()))

        self.assertEqual(result["action"], "reconciled")
        self.assertEqual(result["health_after"]["status"], "ok")

    def test_systemd_projection_targets_controller_entrypoint(self):
        service_text = Path("infra/systemd/organ-lifecycle.service").read_text()
        timer_text = Path("infra/systemd/organ-lifecycle.timer").read_text()
        manifest_text = Path("organs/lifecycle/MANIFEST.yaml").read_text()

        self.assertIn("organs.lifecycle.src.entrypoints.controller", service_text)
        self.assertIn("PYTHONPATH=@CYNIC_PROJECT_ROOT@", service_text)
        self.assertIn("X_ORGAN_DIR=@CYNIC_PROJECT_ROOT@/organs/lifecycle/state", service_text)
        self.assertIn("OnUnitActiveSec=15min", timer_text)
        self.assertIn("WantedBy=timers.target", timer_text)
        self.assertIn("infra/systemd/organ-lifecycle.service", manifest_text)
        self.assertIn("infra/systemd/organ-lifecycle.timer", manifest_text)

    def test_makefile_exposes_organ_lifecycle_install_target(self):
        makefile = Path("Makefile").read_text()

        self.assertIn(".PHONY: organ-lifecycle-install", makefile)
        self.assertIn("systemctl link \"$(PROJECT_DIR)/infra/systemd/organ-lifecycle.service\"", makefile)
        self.assertIn("systemctl enable --now organ-lifecycle.timer", makefile)

    def test_makefile_exposes_organ_lifecycle_status_target(self):
        makefile = Path("Makefile").read_text()

        self.assertIn(".PHONY: organ-lifecycle-status", makefile)
        self.assertIn("journalctl -u organ-lifecycle.service -n 50 --no-pager", makefile)

    def test_makefile_exposes_organ_lifecycle_run_target(self):
        makefile = Path("Makefile").read_text()

        self.assertIn(".PHONY: organ-lifecycle-run", makefile)
        self.assertIn("python3 $(PROJECT_DIR)/organs/lifecycle/src/entrypoints/controller.py", makefile)

    def test_makefile_exposes_organ_lifecycle_health_target(self):
        makefile = Path("Makefile").read_text()

        self.assertIn(".PHONY: organ-lifecycle-health", makefile)
        self.assertIn("python3 $(PROJECT_DIR)/organs/lifecycle/src/entrypoints/controller.py --health", makefile)

    def test_makefile_exposes_organ_lifecycle_analyze_target(self):
        makefile = Path("Makefile").read_text()

        self.assertIn(".PHONY: organ-lifecycle-analyze", makefile)
        self.assertIn(
            "python3 $(PROJECT_DIR)/organs/lifecycle/src/entrypoints/cli.py analyze-chain --registry \"$(PROJECT_DIR)/infra/registry.json\"",
            makefile,
        )

    def test_makefile_exposes_organ_lifecycle_watchdog_target(self):
        makefile = Path("Makefile").read_text()

        self.assertIn(".PHONY: organ-lifecycle-watchdog", makefile)
        self.assertIn("python3 $(PROJECT_DIR)/organs/lifecycle/src/entrypoints/watchdog.py", makefile)

    def test_makefile_install_enables_watchdog_timers(self):
        makefile = Path("Makefile").read_text()

        self.assertIn("organ-lifecycle-watchdog.service", makefile)
        self.assertIn("organ-lifecycle-watchdog.timer", makefile)

    def test_makefile_exposes_organ_lifecycle_prove_target(self):
        makefile = Path("Makefile").read_text()

        self.assertIn(".PHONY: organ-lifecycle-prove", makefile)
        self.assertIn("organ-lifecycle-health", makefile)
        self.assertIn("organ-lifecycle-analyze", makefile)
        self.assertIn("organ-lifecycle-run", makefile)
        self.assertIn("organ-lifecycle-watchdog", makefile)
        self.assertIn("organ-lifecycle-status", makefile)


if __name__ == "__main__":
    unittest.main()
