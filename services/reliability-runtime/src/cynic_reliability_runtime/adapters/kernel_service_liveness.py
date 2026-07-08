from __future__ import annotations

import logging
import requests
from typing import Optional

from cynic_reliability_runtime.ports.service_liveness import ServiceLivenessPort


logger = logging.getLogger("kernel_service_liveness_adapter")


class KernelServiceLivenessMixin(ServiceLivenessPort):
    kernel_addr: str

    def _headers(self) -> dict[str, str]:
        raise NotImplementedError

    def service_register(
        self,
        service_id: str,
        tier: int,
        node: str,
        capabilities: list,
        version: Optional[str] = None,
        ttl_secs: int = 120,
    ) -> bool:
        try:
            payload = {
                "service_id": service_id,
                "tier": tier,
                "node": node,
                "capabilities": capabilities,
                "version": version,
                "ttl_secs": ttl_secs,
            }
            resp = requests.post(
                f"{self.kernel_addr}/services/register",
                headers=self._headers(),
                json=payload,
                timeout=5,
            )
            if resp.status_code >= 400:
                logger.warning("service_register rejected: HTTP %s", resp.status_code)
                return False
            logger.info("service registered in kernel process table: %s", service_id)
            return True
        except requests.RequestException as e:
            logger.debug("service_register unreachable (fire-and-forget): %s", e)
            return False

    def service_heartbeat(self, service_id: str) -> bool:
        try:
            resp = requests.post(
                f"{self.kernel_addr}/services/{service_id}/heartbeat",
                headers=self._headers(),
                timeout=3,
            )
            if resp.status_code == 404:
                logger.warning("service_heartbeat: %s not registered, need re-register", service_id)
                return False
            return resp.status_code < 400
        except requests.RequestException as e:
            logger.debug("service_heartbeat unreachable: %s", e)
            return False

    def service_unregister(self, service_id: str) -> bool:
        try:
            resp = requests.delete(
                f"{self.kernel_addr}/services/{service_id}",
                headers=self._headers(),
                timeout=3,
            )
            if resp.status_code < 400:
                logger.info("service unregistered from kernel process table: %s", service_id)
                return True
            return False
        except requests.RequestException as e:
            logger.debug("service_unregister unreachable (kernel will TTL-evict): %s", e)
            return False
