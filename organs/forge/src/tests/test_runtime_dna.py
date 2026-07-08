import subprocess
import tempfile
import unittest
from pathlib import Path

from organs.forge.src.core.runtime_dna import build_runtime_dna_report


class RuntimeDnaReportTests(unittest.TestCase):
    def test_clean_runtime_checkout_reports_ok(self):
        outputs = {
            ("git", "rev-parse", "HEAD"): "abc123\n",
            ("git", "branch", "--show-current"): "main\n",
            (
                "git",
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{u}",
            ): "origin/main\n",
            ("git", "remote", "-v"): "origin\thttps://example.invalid/CYNIC.git (fetch)\n",
            ("git", "status", "--porcelain"): "",
        }

        def runner(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, stdout=outputs[tuple(command)], stderr="")

        report = build_runtime_dna_report("/repo", runner=runner)

        self.assertEqual(report["status"], "ok")
        self.assertEqual(report["dirty_count"], 0)
        self.assertEqual(report["head"], "abc123")
        self.assertEqual(report["upstream"], "origin/main")

    def test_dirty_runtime_checkout_reports_degraded_without_resetting(self):
        outputs = {
            ("git", "rev-parse", "HEAD"): "abc123\n",
            ("git", "branch", "--show-current"): "feature\n",
            (
                "git",
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{u}",
            ): "origin/feature\n",
            ("git", "remote", "-v"): "",
            ("git", "status", "--porcelain"): " M crates/cynic-mcp/src/proxy.rs\n?? tmp/file\n",
        }

        def runner(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, stdout=outputs[tuple(command)], stderr="")

        report = build_runtime_dna_report("/repo", runner=runner)

        self.assertEqual(report["status"], "degraded")
        self.assertEqual(report["dirty_count"], 2)
        self.assertFalse(report["sync_policy"]["destructive_reset_allowed"])
        self.assertEqual(
            [
                "diagnose_runtime_dna",
                "classify_dirty_runtime",
                "archive_or_promote_local_matter",
                "select_canonical_sync_source",
                "fast_forward_or_block",
                "build",
                "read_back",
            ],
            report["sync_policy"]["required_preflight_order"],
        )
        self.assertIn("fast_forward_only", report["sync_policy"]["allowed_sync_modes"])

    def test_incomplete_git_materialization_reports_specific_blocker(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo = Path(tmpdir)
            (repo / ".git").mkdir()
            (repo / ".git" / "HEAD").write_text("ref: refs/heads/main\n", encoding="utf-8")
            (repo / ".git" / "config").write_text("[core]\n\tbare = false\n", encoding="utf-8")

            def runner(command, **kwargs):
                return subprocess.CompletedProcess(
                    command,
                    128,
                    stdout="",
                    stderr="fatal: not a git repository (or any of the parent directories): .git\n",
                )

            report = build_runtime_dna_report(repo, runner=runner)

        self.assertEqual(report["status"], "blocked")
        self.assertEqual(report["summary"], "runtime checkout has incomplete .git materialization")
        self.assertEqual(report["repo_probe"]["blocked_reason"], "git_objects_missing")
        self.assertFalse(report["repo_probe"]["git_objects_exists"])


if __name__ == "__main__":
    unittest.main()
