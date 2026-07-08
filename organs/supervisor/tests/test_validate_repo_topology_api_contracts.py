import json
import tempfile
import unittest
from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "controllers" / "validate_repo_topology.py"


def load_module():
    return load_module_from_path("validate_repo_topology", MODULE_PATH)


class ApiAndMetaContractTopologyTests(unittest.TestCase):
    def test_validate_pipeline_includes_meta_layer_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.host.validate_meta_layer_contract
        sentinel = module.Finding(
            "critical",
            "meta_layer_contract_pipeline_sentinel",
            "axioms/meta-layer-contract-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.host.validate_meta_layer_contract = fake_validate
            findings = module.validate()
        finally:
            policy.host.validate_meta_layer_contract = original

        self.assertIn(sentinel, findings)

    def test_validate_pipeline_includes_api_layer_alignment_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.host.validate_api_layer_alignment_contract
        sentinel = module.Finding(
            "critical",
            "api_layer_alignment_pipeline_sentinel",
            "axioms/api-layer-alignment-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.host.validate_api_layer_alignment_contract = fake_validate
            findings = module.validate()
        finally:
            policy.host.validate_api_layer_alignment_contract = original

        self.assertIn(sentinel, findings)

    def test_validate_pipeline_includes_api_topology_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.host.validate_api_topology_contract
        sentinel = module.Finding(
            "critical",
            "api_topology_pipeline_sentinel",
            "axioms/api-topology-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.host.validate_api_topology_contract = fake_validate
            findings = module.validate()
        finally:
            policy.host.validate_api_topology_contract = original

        self.assertIn(sentinel, findings)

    def test_validate_pipeline_includes_openapi_route_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.host.validate_openapi_route_contract
        sentinel = module.Finding(
            "critical",
            "openapi_route_pipeline_sentinel",
            "docs/api/openapi.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.host.validate_openapi_route_contract = fake_validate
            findings = module.validate()
        finally:
            policy.host.validate_openapi_route_contract = original

        self.assertIn(sentinel, findings)

    def test_openapi_route_contract_rejects_router_path_missing_from_openapi(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            router_path = root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "mod.rs"
            openapi_path = root / "docs" / "api" / "openapi.json"
            router_path.parent.mkdir(parents=True)
            openapi_path.parent.mkdir(parents=True)
            router_path.write_text(
                'Router::new().route("/health", get(health)).route("/missing", get(missing));\n',
                encoding="utf-8",
            )
            openapi_path.write_text(
                json.dumps({"openapi": "3.1.0", "paths": {"/health": {"get": {}}}}),
                encoding="utf-8",
            )

            from organs.supervisor.src.core.openapi_route_contract import (
                OpenApiRouteContractService,
            )

            service = OpenApiRouteContractService(
                workspace_root=root,
                router_path=router_path,
                openapi_path=openapi_path,
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        self.assertIn("openapi_route_missing", {item.kind for item in findings})

    def test_api_topology_rejects_openapi_operation_without_topology_entry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (root / "docs" / "api").mkdir(parents=True)
            (root / "crates" / "cynic-kernel" / "src" / "api" / "rest").mkdir(parents=True)
            (root / "docs" / "api" / "openapi.json").write_text(
                json.dumps(
                    {
                        "openapi": "3.1.0",
                        "x-cynic-api-topology-contract": {
                            "schema": "cynic.api_topology.v1",
                            "path": "axioms/api-topology-v1.json",
                        },
                        "x-cynic-api-layer-alignment-contract": {
                            "schema": "cynic.api_layer_alignment.v1",
                            "path": "axioms/api-layer-alignment-v1.json",
                        },
                        "paths": {
                            "/openapi.json": {"get": {"security": []}},
                            "/api/topology": {"get": {"security": []}},
                            "/api/layer-alignment": {"get": {"security": []}},
                            "/unmapped": {"get": {"security": []}},
                        },
                    }
                ),
                encoding="utf-8",
            )
            (contracts / "api-layer-alignment-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.api_layer_alignment.v1",
                        "status": "normative",
                        "symbolic_vocabulary": {"Hod": "interface"},
                        "layers": {
                            "perception": {
                                "engineering_role": "test",
                                "agent_question": "test?",
                                "system_primitives": ["read", "poll", "file descriptor"],
                                "symbolic_alignment": ["Hod"],
                                "must": ["test"],
                                "must_not": ["test"],
                            }
                        },
                    }
                ),
                encoding="utf-8",
            )
            discovery_ops = {
                "GET /openapi.json": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": True,
                    "canonical_family": "/api",
                    "status": "canonical",
                },
                "GET /api/topology": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": True,
                    "canonical_family": "/api",
                    "status": "canonical",
                },
                "GET /api/layer-alignment": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": True,
                    "canonical_family": "/api",
                    "status": "canonical",
                },
            }
            (contracts / "api-topology-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.api_topology.v1",
                        "status": "normative",
                        "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
                        "allowed": {
                            "layers": ["perception"],
                            "mutation_levels": ["read"],
                            "statuses": ["canonical"],
                        },
                        "topology_budget": {
                            "max_root_splits": 0,
                            "max_transitional_operations": 0,
                            "max_legacy_operations": 0,
                            "compat_operations": {"allowed": False},
                        },
                        "public_discovery_endpoints": {
                            "GET /openapi.json": {"serves": "docs/api/openapi.json"},
                            "GET /api/topology": {"serves": "axioms/api-topology-v1.json"},
                            "GET /api/layer-alignment": {"serves": "axioms/api-layer-alignment-v1.json"},
                        },
                        "operations": discovery_ops,
                    }
                ),
                encoding="utf-8",
            )
            (root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "discovery.rs").write_text(
                'const OPENAPI_JSON: &str = include_str!("../../../../../docs/api/openapi.json");\n'
                'const API_TOPOLOGY_JSON: &str = include_str!("../../../../../axioms/api-topology-v1.json");\n'
                'const API_LAYER_ALIGNMENT_JSON: &str = include_str!("../../../../../axioms/api-layer-alignment-v1.json");\n'
                "pub async fn openapi_handler() -> Response { json_contract(OPENAPI_JSON) }\n"
                "pub async fn api_topology_handler() -> Response { json_contract(API_TOPOLOGY_JSON) }\n"
                "pub async fn api_layer_alignment_handler() -> Response { json_contract(API_LAYER_ALIGNMENT_JSON) }\n",
                encoding="utf-8",
            )
            (root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "middleware.rs").write_text(
                'path == "/openapi.json" || path == "/api/topology" || path == "/api/layer-alignment"\n',
                encoding="utf-8",
            )

            from organs.supervisor.src.core.api_topology_contract import ApiTopologyContractService

            service = ApiTopologyContractService(
                workspace_root=root,
                openapi_path=root / "docs" / "api" / "openapi.json",
                topology_path=contracts / "api-topology-v1.json",
                alignment_path=contracts / "api-layer-alignment-v1.json",
                discovery_rs_path=root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "discovery.rs",
                middleware_rs_path=root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "middleware.rs",
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        self.assertIn("api_topology_operation_missing", {item.kind for item in findings})

    def test_api_geometry_report_uses_supervisor_topology_primitives(self):
        from organs.supervisor.src.core.api_topology_contract import api_geometry_report

        openapi = {
            "paths": {
                "/legacy/a": {"get": {}},
                "/legacy/b": {"get": {}},
                "/lifecycle/deploy": {"post": {}},
            }
        }
        topology = {
            "topology_budget": {"max_root_splits": 0, "max_transitional_operations": 0},
            "operations": {
                "GET /legacy/a": {
                    "layer": "compatibility",
                    "mutation_level": "read",
                    "agent_safe_default": True,
                    "canonical_family": "/canonical-a",
                    "status": "transitional",
                },
                "GET /legacy/b": {
                    "layer": "compatibility",
                    "mutation_level": "read",
                    "agent_safe_default": True,
                    "canonical_family": "/canonical-b",
                    "status": "canonical",
                },
                "POST /lifecycle/deploy": {
                    "layer": "metabolism",
                    "mutation_level": "deployment",
                    "agent_safe_default": False,
                    "canonical_family": "/lifecycle",
                    "status": "canonical",
                },
            },
        }

        report = api_geometry_report(
            openapi=openapi,
            topology=topology,
            openapi_source="docs/api/openapi.json",
            topology_source="axioms/api-topology-v1.json",
        )

        self.assertEqual("cynic.api_geometry_report.v1", report["schema"])
        self.assertEqual(1, report["summary"]["root_split_count"])
        self.assertEqual(1, report["summary"]["transitional_operation_count"])
        self.assertEqual(1, report["summary"]["dangerous_operation_count"])
        self.assertEqual(["/canonical-a"], list(report["transitional_by_family"]))

    def test_api_layer_alignment_rejects_symbolic_vocabulary_drift(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "api-layer-alignment-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.api_layer_alignment.v1",
                        "status": "normative",
                        "symbolic_vocabulary": {"Keter": "source"},
                        "guardrails": ["analogy stays below evidence"],
                        "layers": {
                            "perception": {
                                "engineering_role": "test",
                                "agent_question": "test?",
                                "system_primitives": ["read", "poll", "file descriptor"],
                                "symbolic_alignment": ["Keter"],
                                "must": ["test"],
                                "must_not": ["test"],
                            }
                        },
                        "vertical_alignment_rules": [{"id": "test", "rule": "test", "enforcement": "validator"}],
                    }
                ),
                encoding="utf-8",
            )
            (contracts / "api-topology-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.api_topology.v1",
                        "status": "normative",
                        "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
                        "allowed": {"layers": ["perception"]},
                        "operations": {},
                    }
                ),
                encoding="utf-8",
            )
            peer = {
                "schema": "test",
                "status": "normative",
                "symbolic_vocabulary": {
                    "Keter": "source",
                    "Binah": "bounded form",
                },
            }
            (contracts / "meta-layer-contract-v1.json").write_text(json.dumps(peer), encoding="utf-8")
            (contracts / "organ-layer-topology-v1.json").write_text(json.dumps(peer), encoding="utf-8")

            from organs.supervisor.src.core.api_layer_alignment_contract import (
                ApiLayerAlignmentContractService,
            )

            service = ApiLayerAlignmentContractService(
                workspace_root=root,
                contract_path=contracts / "api-layer-alignment-v1.json",
                topology_path=contracts / "api-topology-v1.json",
                meta_layer_path=contracts / "meta-layer-contract-v1.json",
                organ_layer_path=contracts / "organ-layer-topology-v1.json",
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        self.assertIn(
            "api_layer_symbolic_vocabulary_missing_peer_symbols",
            {item.kind for item in findings},
        )

    def test_meta_layer_contract_rejects_missing_system_primitives(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "axioms" / "meta-layer-contract-v1.json"
            contract_path.parent.mkdir(parents=True)
            contract = {
                "schema": "cynic.meta_layer_contract.v1",
                "status": "normative",
                "recursion_boundary": {
                    "rule": "test root",
                    "root_contract": "axioms/meta-layer-contract-v1.json",
                    "registry_contract": "axioms/axiom-registry-v1.json",
                    "allowed_meta_contracts": [
                        "axioms/meta-layer-contract-v1.json",
                        "axioms/axiom-registry-v1.json",
                    ],
                    "projection_rule": "test projection",
                    "new_contract_admission": ["classify in registry"],
                    "forbidden": ["competing contract root"],
                },
                "symbolic_vocabulary": {"Keter": "source"},
                "precedence": ["policy"],
                "layers": [
                    {
                        "name": "policy",
                        "canonical_home": "docs/foundation/",
                        "engineering_role": "test",
                        "agent_question": "test?",
                        "symbolic_alignment": ["Keter"],
                        "may_decide": ["test"],
                    }
                ],
            }
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            from organs.supervisor.src.core.meta_layer_contract import MetaLayerContractService

            service = MetaLayerContractService(
                workspace_root=root,
                contract_path=contract_path,
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        self.assertIn("meta_layer_metadata_missing", {item.kind for item in findings})
        self.assertIn("meta_layer_system_primitives_too_weak", {item.kind for item in findings})

    def test_meta_layer_contract_rejects_missing_recursion_boundary(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "axioms" / "meta-layer-contract-v1.json"
            contract_path.parent.mkdir(parents=True)
            contract = {
                "schema": "cynic.meta_layer_contract.v1",
                "status": "normative",
                "symbolic_vocabulary": {"Keter": "source"},
                "precedence": ["policy"],
                "layers": [
                    {
                        "name": "policy",
                        "canonical_home": "docs/foundation/",
                        "engineering_role": "test",
                        "agent_question": "test?",
                        "system_primitives": ["policy file", "review boundary", "admission criterion"],
                        "symbolic_alignment": ["Keter"],
                        "may_decide": ["test"],
                    }
                ],
            }
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            from organs.supervisor.src.core.meta_layer_contract import MetaLayerContractService

            service = MetaLayerContractService(
                workspace_root=root,
                contract_path=contract_path,
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        self.assertIn("meta_layer_recursion_boundary_missing", {item.kind for item in findings})
