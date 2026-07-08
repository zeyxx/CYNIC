from __future__ import annotations

import unittest

from organs.lifecycle.src.adapters.proxmox.bootstrap_builder import BootstrapBuilder


class ProxmoxBootstrapBuilderTests(unittest.TestCase):
    def test_generate_synapse_cloud_init_bootstraps_wireguard_without_legacy_daemon(self):
        payload = BootstrapBuilder.generate_synapse_cloud_init(
            mesh_ip="10.20.0.42",
            public_key_post_url="http://kernel.internal/mesh/publish",
        )

        self.assertIn("wireguard-tools", payload)
        self.assertIn("Address = 10.20.0.42/24", payload)
        self.assertIn("git clone https://github.com/zeyxx/CYNIC.git /opt/cynic", payload)
        self.assertNotIn("organ-wireguard-daemon.service", payload)
        self.assertNotIn("/opt/cynic/organs/wireguard", payload)
        self.assertNotIn("uv sync", payload)


if __name__ == "__main__":
    unittest.main()
