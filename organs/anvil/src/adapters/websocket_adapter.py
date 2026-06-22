import asyncio
import os
import json
import logging
import websockets
from typing import Dict, Any

from src.core.entities import BuildContext, TargetEnvironment
from src.core.services import BuildService

logger = logging.getLogger(__name__)

class KernelWebSocketAdapter:
    def __init__(self, uri: str, api_key: str, public_key: str, identity: str, build_service: BuildService):
        self.uri = uri
        self.api_key = api_key
        self.public_key = public_key
        self.identity = identity
        self.build_service = build_service
        self.ws = None
        self.default_project_root = os.environ.get(
            "KYN_ANVIL_PROJECT_ROOT",
            "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC",
        )
        self.default_kotlin_command = (
            "./gradlew "
            "--no-daemon "
            "-Dorg.gradle.workers.max=1 "
            "-Dkotlin.compiler.execution.strategy=in-process "
            "-Dkotlin.incremental=false "
            ":app:assembleDebug"
        )

    def _parse_task_payload(self, patch: str, intent: str) -> Dict[str, Any]:
        payload: Dict[str, Any] = {}

        if isinstance(intent, str):
            intent_text = intent.strip()
            if intent_text.startswith("{") and intent_text.endswith("}"):
                try:
                    parsed = json.loads(intent_text)
                    if isinstance(parsed, dict):
                        payload = parsed
                except json.JSONDecodeError:
                    payload = {"intent": intent_text}
            elif intent_text:
                payload = {"intent": intent_text}

        patch_path = payload.get("patch") if isinstance(payload.get("patch"), str) else patch
        task_kind = payload.get("task") or payload.get("type")
        if isinstance(task_kind, str) and task_kind.lower() in {"kyon", "kyon-build", "kyon-android-build"}:
            command = payload.get("command") or self.default_kotlin_command
            project_root = payload.get("project_root") or self.default_project_root
            env_vars = payload.get("env") if isinstance(payload.get("env"), dict) else {}
            artifact_path = payload.get("artifact_path")
            destination_path = payload.get("destination_path")
        else:
            command = payload.get("command") or "cargo test --workspace --no-run"
            project_root = payload.get("project_root") or self.default_project_root
            env_vars = payload.get("env") if isinstance(payload.get("env"), dict) else {}
            artifact_path = payload.get("artifact_path")
            destination_path = payload.get("destination_path")
            if payload.get("intent") == "kyon-android-build":
                command = self.default_kotlin_command

        if not isinstance(project_root, str) or not project_root:
            project_root = self.default_project_root

        merged_env = {"CARGO_TERM_COLOR": "always"}
        merged_env.update({k: str(v) for k, v in env_vars.items() if isinstance(k, str)})

        return {
            "command": command,
            "patch": patch_path,
            "project_root": project_root,
            "env_vars": merged_env,
            "artifact_path": artifact_path if isinstance(artifact_path, str) else None,
            "destination_path": destination_path if isinstance(destination_path, str) else None,
            "skip_cleanup": bool(payload.get("skip_cleanup", False)),
        }

    async def _emit_task_result(self, task_id: str, status: str, artifact: str | None):
        if not self.ws:
            return
        try:
            await self.ws.send(json.dumps({
                "type": "task_result",
                "task_id": task_id,
                "status": status,
                "artifact": artifact,
            }))
        except Exception as error:
            logger.error("Failed to emit task result for %s: %s", task_id, error)

    async def connect_and_listen(self):
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "X-Node-Public-Key": self.public_key,
            "X-Node-Identity": self.identity,
        }
        
        while True:
            try:
                async with websockets.connect(self.uri, additional_headers=headers) as websocket:
                    self.ws = websocket
                    logger.info(f"Connected to CYNIC Kernel WS at {self.uri}")
                    
                    async for message in websocket:
                        await self._handle_message(message)
            except websockets.ConnectionClosed:
                logger.warning("Connection closed, reconnecting in 2 seconds...")
                await asyncio.sleep(2)
            except Exception as e:
                logger.error(f"WebSocket error: {e}")
                await asyncio.sleep(2)

    async def _handle_message(self, message: str):
        try:
            data = json.loads(message)
            msg_type = data.get("type")
            
            if msg_type == "route_task":
                task_id = data.get("task_id")
                patch = data.get("patch")
                intent = data.get("intent")
                
                # Spawn asynchronously to not block the event loop listening to WS
                asyncio.create_task(self._execute_build(task_id, patch, intent))
                
            elif msg_type == "register_response":
                logger.info(f"Registered successfully: {data.get('node_id')}")
            
            elif msg_type == "ping":
                await self.ws.send(json.dumps({
                    "type": "pong",
                    "timestamp": data.get("timestamp")
                }))
                
        except Exception as e:
            logger.error(f"Failed to handle message: {e}")

    async def execute_build_task(
        self,
        task_id: str,
        patch: str,
        intent: str,
        publish_result: bool = True,
    ) -> Dict[str, Any]:
        payload = self._parse_task_payload(patch, intent)
        logger.info("Starting task %s command=%s", task_id, payload["command"])

        context = BuildContext(
            project_root=payload["project_root"],
            target_env=TargetEnvironment.LOCAL,
            env_vars=payload["env_vars"],
        )

        async def on_log_chunk(chunk: str):
            if self.ws:
                await self.ws.send(
                    json.dumps({
                        "type": "log_chunk",
                        "task_id": task_id,
                        "stdout": chunk,
                    })
                )

        patch_file = None
        patch_text = payload["patch"]

        if patch_text:
            patch_file = f"/tmp/current_build_{task_id.replace(':', '_')}.patch"
            with open(patch_file, "w") as f:
                f.write(patch_text)

            reset_cmd = "git reset --hard HEAD && git clean -fd"
            await self.build_service.executor_port.execute(context, reset_cmd, on_log_chunk)

            apply_cmd = f"git apply {patch_file}"
            apply_result = await self.build_service.executor_port.execute(context, apply_cmd, on_log_chunk)

            if not apply_result.success:
                logger.error("Failed to apply patch for %s", task_id)
                if publish_result:
                    await self._emit_task_result(task_id, "failed", None)
                return {
                    "status": "failed",
                    "task_id": task_id,
                    "exit_code": apply_result.exit_code,
                    "error": "Failed to apply git patch",
                }

        result = await self.build_service.run_build(
            context=context,
            command=payload["command"],
            on_log_chunk=on_log_chunk,
            artifact_path=payload["artifact_path"],
            destination_path=payload["destination_path"],
        )

        if patch_text and not payload["skip_cleanup"]:
            cleanup_cmd = "git reset --hard HEAD && git clean -fd"
            await self.build_service.executor_port.execute(context, cleanup_cmd, on_log_chunk)

        if patch_file:
            try:
                os.remove(patch_file)
            except FileNotFoundError:
                pass

        result_payload = {
            "task_id": task_id,
            "status": "success" if result.success else "failed",
            "exit_code": result.exit_code,
            "artifact": payload["artifact_path"],
            "stdout_tail": result.stdout[-4000:],
            "stderr": result.stderr,
        }

        if publish_result:
            await self._emit_task_result(task_id, result_payload["status"], payload["artifact_path"])

        logger.info("Task %s completed (Success: %s)", task_id, result.success)
        return result_payload

    async def _execute_build(self, task_id: str, patch: str, intent: str):
        return await self.execute_build_task(task_id, patch, intent, publish_result=True)
