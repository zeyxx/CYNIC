import asyncio
import logging
import os
import sys
import json
import time
import urllib.error
import urllib.request

# Configure logging
logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s")

# Add the project root to sys.path if needed
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))

from src.core.services import BuildService
from src.adapters.local_executor import LocalCommandExecutor, LocalWorkspaceSync, LocalArtifactRetriever
from src.adapters.websocket_adapter import KernelWebSocketAdapter


KERNEL_TASK_KIND = os.getenv("ANVIL_KERNEL_TASK_KIND", "kyon")
KERNEL_TASK_DOMAIN = os.getenv("ANVIL_KERNEL_TASK_DOMAIN", "kyon-mobile")
KERNEL_TASK_POLL_SECONDS = int(os.getenv("ANVIL_KERNEL_TASK_POLL_SECONDS", "8"))
KERNEL_TASK_TIMEOUT_SECONDS = int(os.getenv("ANVIL_KERNEL_TASK_TIMEOUT_SECONDS", "900"))


def _kernel_rest_url() -> str:
    raw = os.getenv("CYNIC_REST_ADDR", "127.0.0.1:3030")
    if raw.startswith(("http://", "https://")):
        return raw.rstrip("/")
    return f"http://{raw.rstrip('/')}"


def _kernel_api_headers() -> dict[str, str]:
    api_key = os.getenv("CYNIC_API_KEY", "")
    return {"Authorization": f"Bearer {api_key}"} if api_key else {}


def _kernel_rest_get(path: str):
    url = f"{_kernel_rest_url()}{path}"
    req = urllib.request.Request(url, headers=_kernel_api_headers())
    with urllib.request.urlopen(req, timeout=KERNEL_TASK_TIMEOUT_SECONDS) as response:
        payload = response.read().decode("utf-8", errors="replace")
        return json.loads(payload)


def _kernel_rest_post(path: str, payload: dict | None = None):
    body = b""
    headers = _kernel_api_headers()
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        body = data
        headers["Content-Type"] = "application/json"

    req = urllib.request.Request(
        f"{_kernel_rest_url()}{path}",
        data=body,
        method="POST",
        headers=headers,
    )
    with urllib.request.urlopen(req, timeout=KERNEL_TASK_TIMEOUT_SECONDS) as response:
        payload_text = response.read().decode("utf-8", errors="replace")
        return json.loads(payload_text) if payload_text else {}


async def _poll_kernel_tasks(adapter: KernelWebSocketAdapter):
    logger = logging.getLogger("KernelTaskPoller")
    headers = _kernel_api_headers()
    if not headers.get("Authorization"):
        logger.warning("CYNIC_API_KEY missing: kernel task polling disabled")
        return

    task_agent_id = os.getenv("ANVIL_AGENT_ID", "organ-anvil")
    logger.info(
        "Starting kernel task poller for kind=%s domain=%s", KERNEL_TASK_KIND, KERNEL_TASK_DOMAIN
    )

    start = time.time()
    while True:
        try:
            pending_path = (
                f"/agent-tasks?kind={KERNEL_TASK_KIND}&domain={KERNEL_TASK_DOMAIN}&limit=5"
            )
            payload = await asyncio.to_thread(_kernel_rest_get, pending_path)
            tasks = payload.get("tasks") if isinstance(payload, dict) else []
            if not isinstance(tasks, list):
                tasks = []

            for task in tasks:
                task_id = task.get("id")
                if not task_id:
                    continue

                claim_payload = {"agent_id": task_agent_id}
                claim_path = f"/agent-tasks/{task_id}/claim"
                try:
                    claim = await asyncio.to_thread(_kernel_rest_post, claim_path, claim_payload)
                except Exception as exc:
                    logger.error("Failed to claim task %s: %s", task_id, exc)
                    continue

                if not isinstance(claim, dict) or not claim.get("claimed"):
                    continue

                content = task.get("content") if isinstance(task.get("content"), str) else ""
                try:
                    parsed = json.loads(content) if content else {}
                except json.JSONDecodeError:
                    parsed = {
                        "command": content,
                    }
                if not isinstance(parsed, dict):
                    parsed = {"command": content}

                intent = json.dumps(parsed)
                patch = parsed.get("patch") if isinstance(parsed.get("patch"), str) else ""

                result = await adapter.execute_build_task(
                    task_id=task_id,
                    patch=patch,
                    intent=intent,
                    publish_result=False,
                )

                complete_payload = {
                    "result": json.dumps(result),
                }
                complete_path = f"/agent-tasks/{task_id}/result"
                await asyncio.to_thread(_kernel_rest_post, complete_path, complete_payload)
                logger.info("Completed kernel task %s status=%s", task_id, result.get("status"))

        except urllib.error.HTTPError as exc:
            logger.error("Kernel task poller HTTP error: %s", exc)
        except urllib.error.URLError as exc:
            logger.error("Kernel task poller network error: %s", exc)
            if time.time() - start > KERNEL_TASK_TIMEOUT_SECONDS:
                logger.warning(
                    "Kernel task poller gave up after %ss due to repeated network errors",
                    KERNEL_TASK_TIMEOUT_SECONDS,
                )
                return
        except Exception as exc:
            logger.error("Kernel task poller error: %s", exc)

        await asyncio.sleep(KERNEL_TASK_POLL_SECONDS)

async def main():
    logger = logging.getLogger("KernelWorker")
    logger.info("Initializing Organ-Anvil Worker...")

    # Wire up the Hexagonal core
    sync_port = LocalWorkspaceSync()
    executor_port = LocalCommandExecutor()
    retriever_port = LocalArtifactRetriever()
    
    build_service = BuildService(sync_port, executor_port, retriever_port)
    
    # Configuration from SSoT (registry.json)
    registry_path = os.path.join(os.path.dirname(__file__), "../../../../infra/registry.json")
    with open(registry_path) as f:
        registry = json.load(f)
        
    kernel_candidates = [
        n for n in registry.get("nodes", []) if n.get("role") in {"kernel_primary", "proxmox_host_kernel"}
    ]
    if not kernel_candidates:
        raise RuntimeError("Could not find kernel node role (kernel_primary/proxmox_host_kernel) in registry.json")
    kernel_node = kernel_candidates[0]
    kernel_node_name = kernel_node.get("name", "<unnamed>")
        
    kernel_ip = kernel_node.get("network", {}).get("tailscale") or kernel_node.get("network", {}).get("local", "")
    if kernel_ip.startswith('<') and kernel_ip.endswith('>'):
        env_var = kernel_ip[1:-1]
        kernel_ip = os.environ.get(env_var, kernel_ip)
    if not kernel_ip or kernel_ip.startswith('<'):
        raise RuntimeError(f"Kernel node '{kernel_node_name}' has no usable tailscale/local address")
    kernel_uri = f"ws://{kernel_ip}:3030/node/ws"

    api_key = os.getenv("CYNIC_API_KEY", "cynic-dev-key")
    public_key = os.getenv("CYNIC_NODE_PUBKEY", "test-pubkey")
    identity = "organ-anvil" # Identity is derived from the organ itself

    # Wire up the adapter
    ws_adapter = KernelWebSocketAdapter(
        uri=kernel_uri,
        api_key=api_key,
        public_key=public_key,
        identity=identity,
        build_service=build_service
    )

    poller_enabled = os.getenv("ANVIL_ENABLE_TASK_POLLER", "1")
    if poller_enabled == "1":
        logger.info("Starting kernel WS listener + kernel task poller")
        await asyncio.gather(
            ws_adapter.connect_and_listen(),
            _poll_kernel_tasks(ws_adapter),
        )
    else:
        logger.info("Starting kernel WS listener only")
        await ws_adapter.connect_and_listen()

if __name__ == "__main__":
    asyncio.run(main())
