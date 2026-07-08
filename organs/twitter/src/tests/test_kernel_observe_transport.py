#!/usr/bin/env python3
import unittest
from unittest import mock

from organs.twitter.src.adapters.kernel_observe_transport import kernel_observe_url, post_observe_payload


class KernelObserveTransportTests(unittest.TestCase):
    def test_kernel_observe_url_normalizes_addr(self):
        self.assertEqual("http://kernel:3030/observe", kernel_observe_url("kernel:3030"))
        self.assertEqual("https://kernel/observe", kernel_observe_url("https://kernel/"))
        self.assertEqual("", kernel_observe_url(""))

    def test_post_observe_payload_skips_without_kernel_env(self):
        with mock.patch.dict("os.environ", {}, clear=True):
            result = post_observe_payload({"tool": "test"})

        self.assertFalse(result.posted)
        self.assertTrue(result.skipped)
        self.assertEqual("kernel env not set", result.error)


if __name__ == "__main__":
    unittest.main()
