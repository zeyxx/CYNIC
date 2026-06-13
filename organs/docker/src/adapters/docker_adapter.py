import subprocess
import json
from pathlib import Path
from typing import List
from src.core.ports import DockerPort
from src.core.entities import Container

EXPECTED_COMPOSE_DIRS = [
    Path(__file__).resolve().parents[4] / "infra/docker/cynic-portal"
]

class SystemDockerAdapter(DockerPort):
    def _run_sudo_docker(self, *args):
        cmd = ["sudo", "-n", "docker"] + list(args)
        res = subprocess.run(cmd, capture_output=True, text=True)
        if res.returncode != 0:
            print(f"Docker command failed: {res.stderr}")
            return None
        return res.stdout.strip()

    def _run_sudo_docker_compose(self, compose_dir: Path, *args):
        cmd = ["sudo", "-n", "docker", "compose", "-f", str(compose_dir / "docker-compose.yml")] + list(args)
        res = subprocess.run(cmd, capture_output=True, text=True)
        return res.returncode == 0

    def get_all_containers(self) -> List[Container]:
        out = self._run_sudo_docker("ps", "-a", "--format", "{{json .}}")
        if not out:
            return []
        
        containers = []
        for line in out.splitlines():
            if not line.strip(): continue
            try:
                raw = json.loads(line)
                status = raw.get("Status", "")
                name = raw.get("Names", "")
                containers.append(Container(
                    raw_data=raw,
                    name=name,
                    status=status,
                    is_up="Up" in status
                ))
            except json.JSONDecodeError:
                pass
        return containers

    def heal_service(self, compose_dir_name: str) -> bool:
        for compose_dir in EXPECTED_COMPOSE_DIRS:
            if compose_dir.name == compose_dir_name and compose_dir.exists():
                return self._run_sudo_docker_compose(compose_dir, "up", "-d")
        return False
