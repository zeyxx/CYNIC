#!/usr/bin/env python3
import unittest
from unittest import mock

from organs.browser.src.adapters.hub_status import read_browser_hub_status


class FakeResponse:
    def __init__(self, status: int, body: str):
        self.status = status
        self._body = body.encode("utf-8")

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        return False

    def read(self) -> bytes:
        return self._body


class BrowserHubStatusTests(unittest.TestCase):
    def test_read_browser_hub_status_reports_chrome_reachable(self):
        with mock.patch(
            "organs.browser.src.adapters.hub_status.urllib.request.urlopen",
            return_value=FakeResponse(200, '{"chrome":"reachable"}'),
        ):
            status = read_browser_hub_status("http://127.0.0.1:40770")

        self.assertTrue(status.http_ok)
        self.assertTrue(status.chrome_reachable)
        self.assertEqual("reachable", status.chrome_detail)

    def test_read_browser_hub_status_reports_invalid_json(self):
        with mock.patch(
            "organs.browser.src.adapters.hub_status.urllib.request.urlopen",
            return_value=FakeResponse(200, "not-json"),
        ):
            status = read_browser_hub_status("http://127.0.0.1:40770")

        self.assertFalse(status.http_ok)
        self.assertIn("invalid JSON", status.observed)
        self.assertFalse(status.chrome_reachable)


if __name__ == "__main__":
    unittest.main()
