import unittest

from organs.supervisor.src.core.command_surfaces_contract import (
    validate_command_surfaces_contract,
)


def base_contract():
    return {
        "schema": "cynic.command_surfaces.v1",
        "surface_roles": {
            "human_operator": {},
            "interactive_agent": {},
            "machine_fallback": {},
            "validation_dev": {},
            "runtime_daemon": {},
        },
        "commands": [
            {
                "name": "cynic_lifecycle_dispatch",
                "role": "interactive_agent",
                "domain": "lifecycle",
                "canonical": True,
                "mutates": True,
                "mutates_mempool": True,
                "json_output_required": True,
                "read_back_required": True,
            },
            {
                "name": "make lint-organ-memory-contract",
                "role": "validation_dev",
                "domain": "validation",
                "canonical": True,
                "mutates": False,
                "mutates_mempool": False,
                "json_output_required": True,
            },
        ],
    }


class CommandSurfacesContractTests(unittest.TestCase):
    def test_accepts_valid_command_surface_contract(self):
        findings = validate_command_surfaces_contract(
            base_contract(),
            exposure={"mcp_tool_domains": {"lifecycle": {"tools": ["cynic_lifecycle_dispatch"]}}},
            make_targets={"lint-organ-memory-contract"},
        )

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_machine_command_without_json(self):
        contract = base_contract()
        contract["commands"][0]["json_output_required"] = False

        findings = validate_command_surfaces_contract(
            contract,
            exposure={"mcp_tool_domains": {"lifecycle": {"tools": ["cynic_lifecycle_dispatch"]}}},
            make_targets={"lint-organ-memory-contract"},
        )

        self.assertIn("machine_command_without_json_output", [item.kind for item in findings])

    def test_rejects_mempool_mutation_without_read_back(self):
        contract = base_contract()
        contract["commands"][0]["read_back_required"] = False

        findings = validate_command_surfaces_contract(
            contract,
            exposure={"mcp_tool_domains": {"lifecycle": {"tools": ["cynic_lifecycle_dispatch"]}}},
            make_targets={"lint-organ-memory-contract"},
        )

        self.assertIn("mempool_mutation_without_read_back", [item.kind for item in findings])

    def test_rejects_canonical_mutating_lifecycle_make_target(self):
        contract = base_contract()
        contract["commands"].append(
            {
                "name": "make deploy-request",
                "role": "validation_dev",
                "domain": "lifecycle",
                "canonical": True,
                "mutates": True,
                "mutates_mempool": True,
                "json_output_required": False,
                "read_back_required": True,
            }
        )

        findings = validate_command_surfaces_contract(
            contract,
            exposure={"mcp_tool_domains": {"lifecycle": {"tools": ["cynic_lifecycle_dispatch"]}}},
            make_targets={"lint-organ-memory-contract", "deploy-request"},
        )

        self.assertIn("make_lifecycle_mutation_marked_canonical", [item.kind for item in findings])


if __name__ == "__main__":
    unittest.main()
