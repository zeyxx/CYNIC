from enum import Enum
from dataclasses import dataclass, field
from typing import Dict, Optional, List

class TargetEnvironment(Enum):
    LOCAL = "local"
    PROXMOX = "proxmox"
    DOCKER = "docker"

@dataclass
class BuildContext:
    """
    Represents the context required to execute a build or a command.
    """
    project_root: str
    target_env: TargetEnvironment
    target_host: Optional[str] = None  # e.g., IP address or hostname for Proxmox/SSH
    target_user: Optional[str] = None  # e.g., SSH user
    env_vars: Dict[str, str] = field(default_factory=dict)

@dataclass
class BuildResult:
    """
    Represents the outcome of a command execution.
    """
    success: bool
    stdout: str
    stderr: str
    exit_code: int
