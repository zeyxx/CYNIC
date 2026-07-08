#!/usr/bin/env python3
import unittest

from organs.contracts.src.adapters.kernel_observe import post_kernel_observe


class Response:
    status_code = 200


class KernelObserveAdapterTests(unittest.TestCase):
    def test_post_kernel_observe_uses_observe_endpoint(self):
        calls = []

        def fake_post(*args, **kwargs):
            calls.append((args, kwargs))
            return Response()

        result = post_kernel_observe(
            request_post=fake_post,
            kernel_url="http://kernel:3030/",
            auth_key="secret",
            payload={"tool": "test"},
        )

        self.assertTrue(result.posted)
        self.assertEqual("http://kernel:3030/observe", calls[0][0][0])
        self.assertIn("Authorization", calls[0][1]["headers"])

    def test_post_kernel_observe_blocks_without_url(self):
        result = post_kernel_observe(
            request_post=lambda *args, **kwargs: Response(),
            kernel_url="",
            auth_key="secret",
            payload={},
        )

        self.assertFalse(result.posted)
        self.assertEqual("kernel_url_missing", result.error)


if __name__ == "__main__":
    unittest.main()
