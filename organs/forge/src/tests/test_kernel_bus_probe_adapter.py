import unittest
from unittest import mock

from urllib import error

from organs.forge.src.adapters import kernel_bus_probe_adapter as module


class KernelBusProbeAdapterTests(unittest.TestCase):
    def test_diagnose_authenticated_path_stall_when_tcp_is_reachable_and_unauth_returns_401(self):
        fake_http_error = error.HTTPError(
            url="http://10.10.10.140:3030/health",
            code=401,
            msg="Unauthorized",
            hdrs=None,
            fp=None,
        )

        with mock.patch.object(module.socket, "create_connection") as create_connection, mock.patch.object(
            module.request,
            "urlopen",
            side_effect=fake_http_error,
        ):
            create_connection.return_value.__enter__.return_value = None
            diagnosis = module.diagnose_kernel_http_failure(
                base_url="http://10.10.10.140:3030",
            )

        self.assertTrue(diagnosis["tcp_reachable"])
        self.assertEqual(401, diagnosis["unauthenticated_http_status"])
        self.assertEqual("authenticated_path_stalled", diagnosis["failure_class"])

    def test_diagnose_network_unreachable_when_tcp_connect_fails(self):
        with mock.patch.object(
            module.socket,
            "create_connection",
            side_effect=OSError("no route"),
        ):
            diagnosis = module.diagnose_kernel_http_failure(
                base_url="http://10.20.0.5:3030",
            )

        self.assertFalse(diagnosis["tcp_reachable"])
        self.assertEqual("OSError", diagnosis["tcp_error"])
        self.assertEqual("network_unreachable", diagnosis["failure_class"])
