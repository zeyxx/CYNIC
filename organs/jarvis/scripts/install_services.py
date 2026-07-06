#!/usr/bin/env python3
import os
import subprocess
from pathlib import Path

def main():
    print("Installing Jarvis Systemd Services for zero-friction runtime...")
    user_systemd = Path.home() / ".config" / "systemd" / "user"
    user_systemd.mkdir(parents=True, exist_ok=True)
    
    project_root = Path(__file__).parent.parent.parent.parent.absolute()
    
    brain_service = project_root / "organs" / "jarvis" / "systemd" / "cynic-jarvis-brain.service"
    sensor_service = project_root / "organs" / "jarvis" / "systemd" / "cynic-jarvis-sensor.service"
    
    # Symlink services
    for svc in [brain_service, sensor_service]:
        if svc.exists():
            dest = user_systemd / svc.name
            if dest.exists() or dest.is_symlink():
                dest.unlink()
            dest.symlink_to(svc)
    
    subprocess.run(["systemctl", "--user", "daemon-reload"], check=True)
    subprocess.run(["systemctl", "--user", "enable", "--now", "cynic-jarvis-brain.service"], check=True)
    subprocess.run(["systemctl", "--user", "enable", "--now", "cynic-jarvis-sensor.service"], check=True)
    
    print("Jarvis services are now running and will auto-start on boot.")
    subprocess.run(["systemctl", "--user", "status", "cynic-jarvis-brain.service", "--no-pager"], check=False)

if __name__ == "__main__":
    main()
