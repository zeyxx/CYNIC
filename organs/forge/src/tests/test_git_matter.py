import unittest

from organs.forge.src.core.git_matter import (
    DeterministicGitMatterReviewer,
    GitMatterBatch,
    GitMatterReview,
    draft_git_matter_decisions,
)


class GitMatterDecisionDraftTests(unittest.TestCase):
    def test_draft_decisions_consumes_supervisor_batches_without_mutation(self):
        report = {
            "schema": "cynic.git_matter_report.v1",
            "visible_path_total": 3,
            "blockers": [".git is not writable in this environment"],
            "inference_batches": [
                {
                    "batch_id": "git-matter-001",
                    "family": "organs/forge",
                    "topological_class": "organ_body",
                    "status": "untracked",
                    "path_count": 2,
                    "sample_paths": ["organs/forge/src/core/git_matter.py"],
                },
                {
                    "batch_id": "git-matter-002",
                    "family": "crates/cynic-kernel",
                    "topological_class": "kernel_source",
                    "status": "modified",
                    "path_count": 1,
                    "sample_paths": ["crates/cynic-kernel/src/main.rs"],
                },
            ],
        }

        payload = draft_git_matter_decisions(report)

        self.assertEqual("cynic.git_matter_decisions.v1", payload["schema"])
        self.assertEqual("organ-forge", payload["authority"])
        self.assertFalse(payload["mutates_git"])
        self.assertFalse(payload["mutates_mempool"])
        self.assertEqual(2, payload["decision_count"])
        self.assertEqual([".git is not writable in this environment"], payload["blockers"])
        self.assertEqual("medium", payload["decisions"][0]["risk"])
        self.assertEqual("organ", payload["decisions"][0]["owner"])
        self.assertEqual("high", payload["decisions"][1]["risk"])
        self.assertEqual("crate", payload["decisions"][1]["owner"])
        self.assertEqual("cynic.git_matter_decisions.v1", payload["surface_contract"]["schema"])

    def test_rejects_wrong_source_schema(self):
        with self.assertRaisesRegex(RuntimeError, "expected cynic.git_matter_report.v1"):
            draft_git_matter_decisions({"schema": "cynic.git_matter_report.v0"})

    def test_review_port_can_be_replaced_without_changing_core_shape(self):
        class StubReviewer:
            def review_git_matter_batch(self, batch):
                return GitMatterReview(
                    decision="track",
                    risk="low",
                    owner="repo",
                    rationale=f"stub review for {batch.batch_id}",
                    next_probe="none",
                )

        payload = draft_git_matter_decisions(
            {
                "schema": "cynic.git_matter_report.v1",
                "inference_batches": [
                    {
                        "batch_id": "git-matter-001",
                        "family": "docs",
                        "topological_class": "documentation",
                        "status": "modified",
                        "path_count": 1,
                        "sample_paths": ["README.md"],
                    }
                ],
            },
            reviewer=StubReviewer(),
        )

        self.assertEqual("track", payload["decisions"][0]["decision"])
        self.assertEqual("stub review for git-matter-001", payload["decisions"][0]["rationale"])

    def test_default_reviewer_is_conservative_for_unknown_untracked_matter(self):
        review = DeterministicGitMatterReviewer().review_git_matter_batch(
            GitMatterBatch(
                batch_id="git-matter-999",
                family="unknown",
                topological_class="unknown",
                status="untracked",
                path_count=1,
                sample_paths=("tmp/new.py",),
            )
        )

        self.assertEqual("quarantine", review.decision)
        self.assertEqual("low", review.risk)
