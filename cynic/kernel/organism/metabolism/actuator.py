"""
CYNIC Actuator - Physical Code Manipulation & Validation.
Implements the 'Act' phase of the RALPH loop with automated testing.
"""
from __future__ import annotations
import os
import subprocess
import logging
from pathlib import Path

logger = logging.getLogger("cynic.organism.actuator")

class SurgicalActuator:
    def __init__(self, root_dir: str = "."):
        self.root = Path(root_dir)

    async def apply_fix(self, file_path: str, new_content: str) -> bool:
        """
        Attempts to apply a code fix and runs the HeresyGuard.
        Returns True if the fix is stable.
        """
        full_path = self.root / file_path
        old_content = ""
        
        if full_path.exists():
            with open(full_path, "r", encoding="utf-8") as f:
                old_content = f.read()

        try:
            logger.info(f"Actuator: Applying surgery to {file_path}...")
            with open(full_path, "w", encoding="utf-8") as f:
                f.write(new_content)
            
            # Run HeresyGuard (Verify Surgery)
            logger.info("Actuator: Running HeresyGuard validation...")
            result = subprocess.run(
                ["python", "scripts/verify_surgery.py", str(full_path)],
                capture_output=True, text=True
            )
            
            if result.returncode == 0:
                logger.info(f"Actuator: Surgery successful and validated for {file_path}.")
                return True
            else:
                logger.warning(f"Actuator: Surgery REJECTED by HeresyGuard. Rolling back.")
                with open(full_path, "w", encoding="utf-8") as f:
                    f.write(old_content)
                return False
                
        except Exception as e:
            logger.error(f"Actuator: Surgery failed: {e}. Rolling back.")
            if old_content:
                with open(full_path, "w", encoding="utf-8") as f:
                    f.write(old_content)
            return False
