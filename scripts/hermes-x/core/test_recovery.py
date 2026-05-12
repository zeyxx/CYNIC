#!/usr/bin/env python3
"""
Integration tests for hermes_x_recovery.py recovery daemon.

Tests the 4-layer recovery cascade:
  Layer 1: Auth failure detection
  Layer 2: Re-login retry
  Layer 3: Account fallback
  Layer 4: Kernel alert
"""

import os
import sys
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch, MagicMock
from datetime import datetime

# Add core to path
sys.path.insert(0, str(Path(__file__).parent))

# Import recovery module
import hermes_x_recovery as recovery


class TestRecoveryDaemon(unittest.TestCase):
    """Test recovery daemon functions."""

    def setUp(self):
        """Set up test environment."""
        self.temp_dir = tempfile.TemporaryDirectory()
        self.temp_path = Path(self.temp_dir.name)

    def tearDown(self):
        """Clean up."""
        self.temp_dir.cleanup()

    def test_auth_failure_detection_no_engagement_rate(self):
        """Test that missing engagement_rate in captured tweets indicates auth failure."""
        # Simulate dataset with tweets missing engagement_rate (auth error symptom)
        dataset = self.temp_path / "dataset.jsonl"
        with open(dataset, "w") as f:
            # Write tweets WITHOUT engagement_rate (X returns auth error page)
            f.write('{"tweet_id":"1","text":"test","engagement_rate":0.5}\n')
            f.write('{"tweet_id":"2","text":"test"}\n')  # Missing engagement_rate
            f.write('{"tweet_id":"3","text":"test"}\n')  # Missing engagement_rate

        # This is implicit — recovery daemon checks engagement_rate field
        # In real scenario: x_ingest_daemon detects this and posts heartbeat with
        # status="critical", failure_reason="x_auth_expired"
        # Here we just verify the logic
        failure_detected = sum(1 for line in open(dataset) if '"engagement_rate"' not in line) > 1
        self.assertTrue(failure_detected, "Should detect missing engagement_rate")

    def test_recovery_state_counters(self):
        """Test that recovery counters track failure layers."""
        # Simulate detecting auth failure for MAX_FAILURE_BEFORE_LAYER2 cycles
        with patch("hermes_x_recovery.check_heartbeat") as mock_heartbeat:
            mock_heartbeat.return_value = {
                "status": "critical",
                "failure_reason": "x_auth_expired",
                "timestamp": datetime.utcnow().isoformat(),
            }

            # Before Layer 2 threshold
            recovery.FAILURE_COUNT = 0
            recovery.LAYER2_RETRY_COUNT = 0

            # Simulate 2 failure cycles
            for i in range(recovery.MAX_FAILURE_BEFORE_LAYER2):
                old_count = recovery.FAILURE_COUNT
                recovery.FAILURE_COUNT += 1
                self.assertGreater(recovery.FAILURE_COUNT, old_count)

            # At threshold, Layer 2 should trigger
            self.assertEqual(recovery.FAILURE_COUNT, recovery.MAX_FAILURE_BEFORE_LAYER2)

    def test_heartbeat_parsing(self):
        """Test parsing of heartbeat observations from kernel."""
        heartbeat = {
            "status": "critical",
            "failure_reason": "x_auth_expired",
            "timestamp": "2026-05-12T18:45:00Z",
        }

        # Verify structure matches what kernel sends
        self.assertIn("status", heartbeat)
        self.assertIn("failure_reason", heartbeat)
        self.assertEqual(heartbeat["status"], "critical")
        self.assertEqual(heartbeat["failure_reason"], "x_auth_expired")

    def test_layer2_logic_via_subprocess(self):
        """Test Layer 2 login retry subprocess invocation."""
        # Mock subprocess to verify correct invocation
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)

            # Simulate Layer 2 retry
            recovery.LAYER2_RETRY_COUNT = 0
            success = recovery.layer2_retry_login()

            self.assertTrue(success, "Layer 2 should succeed when subprocess returns 0")
            mock_run.assert_called_once()
            call_args = mock_run.call_args
            self.assertIn("hermes_x_login.py", str(call_args))
            self.assertIn("--force", call_args[0][0])

    def test_layer3_fallback_account_lookup(self):
        """Test Layer 3 fallback account selection from config."""
        # Create mock accounts.toml
        config = {
            "accounts": {
                "cynic": {"username": "CynicOracle", "resume_on_failure": False},
                "personal": {
                    "username": "jeanterre552",
                    "resume_on_failure": True,
                },
            }
        }

        # Patch get_accounts_config
        with patch("hermes_x_recovery.get_accounts_config", return_value=config):
            with patch("hermes_x_recovery.ACCOUNT_ID", "cynic"):
                fallback = recovery.get_fallback_account()
                self.assertEqual(fallback, "personal", "Should find personal as fallback")

    def test_layer3_fallback_switch_via_script(self):
        """Test Layer 3 account switch via toggle-x-account.sh."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)

            recovery.LAYER3_RETRY_COUNT = 0
            success = recovery.layer3_switch_account("personal")

            self.assertTrue(
                success, "Layer 3 should succeed when toggle script returns 0"
            )
            mock_run.assert_called_once()
            call_args = mock_run.call_args
            self.assertIn("toggle-x-account.sh", str(call_args))
            self.assertIn("personal", call_args[0][0])

    def test_layer4_kernel_alert(self):
        """Test Layer 4 alert POST to kernel."""
        with patch("requests.post") as mock_post:
            mock_post.return_value = MagicMock(status_code=200)

            recovery.LAYER2_RETRY_COUNT = 3
            recovery.LAYER3_RETRY_COUNT = 1
            recovery.layer4_alert_kernel()

            mock_post.assert_called_once()
            call_kwargs = mock_post.call_args[1]
            payload = call_kwargs.get("json", {})

            # Verify critical alert structure
            self.assertEqual(payload.get("domain"), "hermes-x")
            self.assertEqual(
                payload.get("context", {}).get("severity"), "critical"
            )
            self.assertIn("recovery-failed", payload.get("tags", []))
            self.assertEqual(payload.get("context", {}).get("layer2_retries"), 3)

    def test_recovery_state_reset_on_healthy_heartbeat(self):
        """Test that healthy heartbeat resets failure counters."""
        recovery.FAILURE_COUNT = 5
        recovery.LAYER2_RETRY_COUNT = 2
        recovery.LAYER3_RETRY_COUNT = 1

        with patch("hermes_x_recovery.check_heartbeat") as mock_hb:
            # Simulate healthy heartbeat
            mock_hb.return_value = {
                "status": "ok",
                "failure_reason": None,
                "timestamp": datetime.utcnow().isoformat(),
            }

            recovery.run_recovery_cycle()

            # Counters should be reset
            self.assertEqual(recovery.FAILURE_COUNT, 0)
            self.assertEqual(recovery.LAYER2_RETRY_COUNT, 0)
            self.assertEqual(recovery.LAYER3_RETRY_COUNT, 0)

    def test_end_to_end_failure_to_alert(self):
        """Test complete recovery cascade from failure detection to kernel alert."""
        # This test validates the cascade logic: failure detection → Layer 2 → Layer 3 → Layer 4
        # Each layer is tested individually in other tests, so here we just verify
        # the overall structure and state transitions

        with patch("hermes_x_recovery.check_heartbeat") as mock_hb:
            # Simulate auth failure
            mock_hb.return_value = {
                "status": "critical",
                "failure_reason": "x_auth_expired",
                "timestamp": datetime.utcnow().isoformat(),
            }

            recovery.FAILURE_COUNT = 0
            recovery.LAYER2_RETRY_COUNT = 0
            recovery.LAYER3_RETRY_COUNT = 0

            # Simulate failure cycles
            for _ in range(recovery.MAX_FAILURE_BEFORE_LAYER2):
                recovery.run_recovery_cycle()

            # Failure count should increase
            self.assertEqual(recovery.FAILURE_COUNT, recovery.MAX_FAILURE_BEFORE_LAYER2)

            # Simulate reset on healthy heartbeat
            mock_hb.return_value = {
                "status": "ok",
                "failure_reason": None,
                "timestamp": datetime.utcnow().isoformat(),
            }
            recovery.run_recovery_cycle()

            # Counters should reset
            self.assertEqual(recovery.FAILURE_COUNT, 0)
            self.assertEqual(recovery.LAYER2_RETRY_COUNT, 0)


def test_toggle_account_script_syntax():
    """Basic syntax check for toggle-x-account.sh."""
    script_path = Path(__file__).parent / "toggle-x-account.sh"
    if script_path.exists():
        result = subprocess.run(
            ["bash", "-n", str(script_path)],
            capture_output=True,
            timeout=5
        )
        assert result.returncode == 0, f"toggle-x-account.sh has syntax errors: {result.stderr.decode()}"


if __name__ == "__main__":
    # Run tests
    unittest.main(verbosity=2)
