import unittest

from organs.supervisor.src.core.organ_memory_contract import validate_organ_memory_contract


def base_contract():
    classes = {
        name: name
        for name in [
            "raw",
            "normalized",
            "warehouse",
            "ccm",
            "outbox",
            "deadletter",
            "cache",
            "artifact",
            "checkpoint",
            "local_journal",
            "local_snapshot",
            "desired_projection",
            "local_contract",
            "operator_view",
            "local_doc",
        ]
    }
    return {
        "schema": "cynic.organ_memory.v1",
        "storage_classes": classes,
        "authority_levels": {
            "local_only": "local only",
            "kernel_promotion_candidate": "promotable",
            "derived_from_git_or_kernel": "projection",
            "legacy_debt": "legacy",
        },
        "state_path_classifications": [
            {
                "path": "organs/forge/state/state.json",
                "organ": "organ-forge",
                "class": "local_snapshot",
                "authority": "legacy_debt",
                "backend": "json",
                "promotion_path": "cynic_observe",
                "next_action": "move to untracked state",
            }
        ],
    }


class OrganMemoryContractTests(unittest.TestCase):
    def test_accepts_classified_tracked_state(self):
        findings = validate_organ_memory_contract(
            base_contract(),
            tracked_paths=["organs/forge/state/state.json"],
        )

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_unclassified_tracked_state(self):
        findings = validate_organ_memory_contract(
            base_contract(),
            tracked_paths=["organs/forge/state/audit.jsonl"],
        )

        self.assertIn(
            "tracked_local_memory_without_classification",
            [item.kind for item in findings],
        )

    def test_rejects_local_memory_global_authority(self):
        contract = base_contract()
        contract["authority_levels"]["organism_canonical"] = "forbidden for local state"
        contract["state_path_classifications"][0]["authority"] = "organism_canonical"

        findings = validate_organ_memory_contract(
            contract,
            tracked_paths=["organs/forge/state/state.json"],
        )

        self.assertIn("local_memory_claims_global_authority", [item.kind for item in findings])


if __name__ == "__main__":
    unittest.main()
