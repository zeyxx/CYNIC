"""
CYNIC Architectural Rails Enforcement - The Heresy Guard.

Active enforcement of AGENT_RAILS.md rules. 
Used by AutoSurgeon to validate mutations before suture.

Lentille: Security / Solutions Architect
"""
import ast
import os
from pathlib import Path
from typing import List, Tuple

class HeresyGuard:
    """
    Scans code for architectural violations (heresies).
    """
    def __init__(self):
        self.violations: List[Tuple[str, str]] = []

    def check_file(self, filepath: Path) -> List[Tuple[str, str]]:
        """Run all rail checks on a single file."""
        self.violations = []
        if not filepath.suffix == ".py":
            return []

        try:
            content_bytes = filepath.read_bytes()
            # Rail 1: ASCII ONLY
            self._check_ascii(filepath, content_bytes)
            
            # Use ascii decode to avoid any encoding issues during parsing
            content = content_bytes.decode('ascii', 'ignore')
            tree = ast.parse(content)
            
            # Rail 2: No os.getenv
            self._check_no_getenv(filepath, tree)
            
            # Rail 3: No except: pass
            self._check_no_silent_exceptions(filepath, tree)
            
        except Exception as e:
            self.violations.append((str(filepath), f"Parse error: {e}"))
            
        return self.violations

    def _check_ascii(self, filepath: Path, content: bytes):
        """Rule: No non-ascii characters."""
        for i, byte in enumerate(content):
            if byte > 127:
                self.violations.append((str(filepath), f"Non-ASCII character at byte {i}"))
                break

    def _check_no_getenv(self, filepath: Path, tree: ast.AST):
        """Rule: All env vars via config.py only."""
        if "config.py" in str(filepath):
            return

        for node in ast.walk(tree):
            if isinstance(node, ast.Call):
                if isinstance(node.func, ast.Attribute):
                    if (isinstance(node.func.value, ast.Name) and 
                        node.func.value.id == 'os' and 
                        node.func.attr == 'getenv'):
                        self.violations.append((str(filepath), "Illegal direct os.getenv() call"))

    def _check_no_silent_exceptions(self, filepath: Path, tree: ast.AST):
        """Rule: No except: pass or bare except:."""
        for node in ast.walk(tree):
            if isinstance(node, ast.ExceptHandler):
                # Rail 3a: No bare except:
                if node.type is None:
                    self.violations.append((str(filepath), "Heresy: bare except: detected (must catch Exception)"))
                    continue

                # Rail 3b: No silent pass or empty block
                is_pass = False
                if len(node.body) == 1 and isinstance(node.body[0], ast.Pass):
                    is_pass = True
                
                if is_pass or len(node.body) == 0:
                    self.violations.append((str(filepath), "Heresy: silent except: pass detected"))

def validate_codebase_rails(root_dir: str = "cynic") -> bool:
    """Scan the entire core for heresies."""
    guard = HeresyGuard()
    all_violations = []
    for root, _, files in os.walk(root_dir):
        for file in files:
            if file.endswith(".py"):
                all_violations.extend(guard.check_file(Path(root) / file))
    
    if all_violations:
        print("ARCHITECTURAL ANOMALIES DETECTED:")
        for loc, msg in all_violations:
            print(f"  - {loc}: {msg}")
        return False
    return True
