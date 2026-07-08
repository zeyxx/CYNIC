from __future__ import annotations

import asyncio
from unittest.mock import patch
import unittest

from organs.lifecycle.src.adapters.kernel_observation_adapter import KernelObservationAdapter


class FakeObserveResult:
    def __init__(self, posted: bool = True, status_code: int | None = 200, error: str | None = None):
        self.posted = posted
        self.status_code = status_code
        self.error = error


class KernelObservationAdapterTests(unittest.TestCase):
    def test_observe_delegates_to_shared_kernel_observe_helper(self):
        calls = []

        def fake_post_kernel_observe(**kwargs):
            calls.append(kwargs)
            return FakeObserveResult()

        adapter = KernelObservationAdapter("http://kernel:3030/", api_key="secret", timeout=3.5)

        with patch("organs.lifecycle.src.adapters.kernel_observation_adapter.post_kernel_observe", side_effect=fake_post_kernel_observe):
            result = asyncio.run(adapter.observe({"tool": "test", "domain": "alpha"}))

        self.assertTrue(result["posted"])
        self.assertEqual("http://kernel:3030/observe", result["endpoint"])
        self.assertEqual(1, len(calls))
        self.assertEqual("http://kernel:3030", calls[0]["kernel_url"])
        self.assertEqual("secret", calls[0]["auth_key"])
        self.assertEqual({"tool": "test", "domain": "alpha"}, calls[0]["payload"])
        self.assertEqual(3.5, calls[0]["timeout"])


if __name__ == "__main__":
    unittest.main()
