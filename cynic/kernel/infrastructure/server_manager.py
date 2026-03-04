"""
CYNIC Sovereign Server Manager - Vulkan/APU Process Lifecycle.
Manages llama-server instances with direct hardware control.
Respects SRE, AI Infra, and Robotics lenses.
"""
from __future__ import annotations

import asyncio
import logging
import os
import subprocess
from pathlib import Path
from typing import Dict, List, Optional

logger = logging.getLogger("cynic.infrastructure.server_manager")

class SovereignServerManager:
    """
    Orchestrates the lifecycle of llama-server processes.
    Optimized for Vulkan on Ryzen APUs.
    """
    def __init__(self, model_dir: str = "D:/cynic-models"):
        self.model_dir = Path(model_dir)
        self._processes: Dict[int, subprocess.Popen] = {}
        self.active_port: Optional[int] = None

    async def start_server(self, model_name: str, port: int = 8080) -> bool:
        """Starts a llama-server with Vulkan acceleration."""
        model_path = self.model_dir / model_name
        if not model_path.exists():
            logger.error(f"ServerManager: Model not found at {model_path}")
            return False

        # Build command for Windows + Vulkan (Using identified path)
        binary_path = r"C:\Users\zeyxm\.docker\bin\inference\llama-server.exe"
        cmd = [
            binary_path,
            "-m", str(model_path),
            "-ngl", "99",
            "--port", str(port),
            "--host", "0.0.0.0"
        ]

        try:
            logger.info(f"ServerManager: Launching {model_name} on port {port} (Vulkan)...")
            # Using subprocess.Popen for long-running background process
            process = subprocess.Popen(
                cmd,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                creationflags=subprocess.CREATE_NEW_PROCESS_GROUP
            )
            
            # Wait for server to warm up
            await asyncio.sleep(5)
            
            if process.poll() is None:
                self._processes[port] = process
                self.active_port = port
                logger.info(f"ServerManager: Server active (PID: {process.pid})")
                return True
            else:
                logger.error("ServerManager: Process failed to start immediately.")
                return False
        except Exception as e:
            logger.error(f"ServerManager: Launch failed: {e}")
            return False

    async def stop_server(self, port: int = 8080):
        """Gracefully terminates a server instance."""
        if port in self._processes:
            process = self._processes[port]
            logger.info(f"ServerManager: Stopping server on port {port} (PID: {process.pid})")
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
            del self._processes[port]
            if self.active_port == port:
                self.active_port = None

    async def switch_model(self, new_model_name: str, port: int = 8080):
        """Juggles models by restarting the server with a new soul."""
        await self.stop_server(port)
        return await self.start_server(new_model_name, port)

    async def shutdown_all(self):
        """Cleans up all managed processes."""
        ports = list(self._processes.keys())
        for port in ports:
            await self.stop_server(port)
