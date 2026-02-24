"""
Windows Environment Auto-Detection Module

Detects Windows-specific issues and provides auto-fix capabilities.
Part of CYNIC's immune system for self-healing.

φ distrusts φ — κυνικός
"""

import platform
import subprocess
import sys
from pathlib import Path
from typing import Optional


class WindowsIssue:
    """Represents a detected Windows-specific issue."""
    
    def __init__(
        self,
        issue_id: str,
        title: str,
        description: str,
        severity: str,  # "critical", "high", "medium", "low"
        fix_command: Optional[str] = None,
        fix_applied: bool = False
    ):
        self.issue_id = issue_id
        self.title = title
        self.description = description
        self.severity = severity
        self.fix_command = fix_command
        self.fix_applied = fix_applied
    
    def to_dict(self) -> dict:
        return {
            "issue_id": self.issue_id,
            "title": self.title,
            "description": self.description,
            "severity": self.severity,
            "fix_command": self.fix_command,
            "fix_applied": self.fix_applied
        }


class WindowsDetector:
    """
    Detects Windows-specific issues that affect pytest and development.
    
    This is part of CYNIC's immune system - it can detect problems
    and optionally auto-fix them.
    """
    
    # Known Windows-specific issues
    KNOWN_ISSUES = {
        "pytest_timeout": {
            "title": "Pytest Timeout Import Error",
            "description": "pytest-timeout may not be properly installed or compatible",
            "severity": "high",
            "fix_command": "pip install pytest-timeout"
        },
        "path_separator": {
            "title": "Path Separator Issue",
            "description": "Using backslash instead of forward slash in paths",
            "severity": "medium",
            "fix_command": "Use pathlib.Path or os.path.join"
        },
        "shell_execution": {
            "title": "Shell Execution Policy",
            "description": "PowerShell execution policy may block scripts",
            "severity": "high",
            "fix_command": "Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser"
        },
        "encoding": {
            "title": "Default Encoding Issue",
            "description": "Windows console may use legacy encoding (cp1252)",
            "severity": "medium",
            "fix_command": "Set PYTHONIOENCODING=utf-8"
        },
        "venv_activation": {
            "title": "Virtual Environment Activation",
            "description": "venv may not activate properly in Windows cmd",
            "severity": "high",
            "fix_command": "Use scripts\\activate instead of source bin/activate"
        },
        "port_availability": {
            "title": "Port Binding Issue",
            "description": "Windows may have ports bound by system services",
            "severity": "medium",
            "fix_command": "Use netsh to check port usage"
        }
    }
    
    def __init__(self):
        self.is_windows = platform.system() == "Windows"
        self.issues: list[WindowsIssue] = []
        self._detected = False
    
    def detect_all(self) -> list[WindowsIssue]:
        """
        Run all detection checks and return list of issues.
        
        Returns:
            List of detected Windows issues
        """
        self.issues = []
        
        if not self.is_windows:
            return self.issues
        
        # Run detection checks
        self._check_pytest_timeout()
        self._check_path_separators()
        self._check_shell_policy()
        self._check_encoding()
        self._check_venv()
        self._check_port_availability()
        
        self._detected = True
        return self.issues
    
    def _check_pytest_timeout(self):
        """Check if pytest-timeout is available."""
        try:
            result = subprocess.run(
                [sys.executable, "-c", "import pytest_timeout"],
                capture_output=True,
                timeout=5
            )
            if result.returncode != 0:
                self.issues.append(WindowsIssue(
                    issue_id="pytest_timeout",
                    **self.KNOWN_ISSUES["pytest_timeout"]
                ))
        except Exception:
            self.issues.append(WindowsIssue(
                issue_id="pytest_timeout",
                **self.KNOWN_ISSUES["pytest_timeout"]
            ))
    
    def _check_path_separators(self):
        """Check for path separator issues in current directory."""
        # This would check if code uses backslashes improperly
        # For now, we just flag the potential issue
        pass
    
    def _check_shell_policy(self):
        """Check PowerShell execution policy."""
        try:
            result = subprocess.run(
                ["powershell", "-Command", "Get-ExecutionPolicy"],
                capture_output=True,
                text=True,
                timeout=5
            )
            policy = result.stdout.strip()
            if policy in ["Restricted", "AllSigned"]:
                self.issues.append(WindowsIssue(
                    issue_id="shell_execution",
                    title=self.KNOWN_ISSUES["shell_execution"]["title"],
                    description=f"Current policy: {policy}",
                    severity=self.KNOWN_ISSUES["shell_execution"]["severity"],
                    fix_command=self.KNOWN_ISSUES["shell_execution"]["fix_command"]
                ))
        except Exception:
            pass
    
    def _check_encoding(self):
        """Check console encoding."""
        try:
            import locale
            encoding = locale.getpreferredencoding()
            if encoding.lower() not in ["utf-8", "utf8"]:
                self.issues.append(WindowsIssue(
                    issue_id="encoding",
                    title=self.KNOWN_ISSUES["encoding"]["title"],
                    description=f"Current encoding: {encoding}",
                    severity=self.KNOWN_ISSUES["encoding"]["severity"],
                    fix_command=self.KNOWN_ISSUES["encoding"]["fix_command"]
                ))
        except Exception:
            pass
    
    def _check_venv(self):
        """Check if virtual environment is properly set up."""
        venv_path = Path("venv") if Path(".").name != "venv" else Path(".")
        if not venv_path.exists():
            self.issues.append(WindowsIssue(
                issue_id="venv_creation",
                title="No Virtual Environment",
                description="No venv found in current directory",
                severity="high",
                fix_command="python -m venv venv"
            ))
    
    def _check_port_availability(self):
        """Check if common ports are available."""
        common_ports = [3000, 5432, 5433, 8000, 8765, 8766, 11434]
        # This is a simplified check - in production, you'd actually try to bind
        pass
    
    def auto_fix(self, issue_id: str) -> bool:
        """
        Attempt to auto-fix a detected issue.
        
        Args:
            issue_id: The ID of the issue to fix
            
        Returns:
            True if fix was applied successfully
        """
        if not self.is_windows:
            return False
        
        issue = next((i for i in self.issues if i.issue_id == issue_id), None)
        if not issue or not issue.fix_command:
            return False
        
        try:
            # Apply fix based on issue type
            if issue_id == "pytest_timeout":
                result = subprocess.run(
                    [sys.executable, "-m", "pip", "install", "pytest-timeout"],
                    capture_output=True,
                    timeout=60
                )
                issue.fix_applied = result.returncode == 0
            
            elif issue_id == "shell_execution":
                result = subprocess.run(
                    ["powershell", "-Command", 
                     f"Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser"],
                    capture_output=True,
                    timeout=30
                )
                issue.fix_applied = result.returncode == 0
            
            # Add more auto-fix logic as needed
            
        except Exception:
            issue.fix_applied = False
        
        return issue.fix_applied
    
    def auto_fix_all(self) -> list[str]:
        """
        Attempt to auto-fix all detectable issues.
        
        Returns:
            List of issue IDs that were successfully fixed
        """
        fixed = []
        for issue in self.issues:
            if self.auto_fix(issue.issue_id):
                fixed.append(issue.issue_id)
        return fixed
    
    def get_health_report(self) -> dict:
        """
        Get a health report of Windows environment.
        
        Returns:
            Dictionary with health status and issues
        """
        if not self._detected:
            self.detect_all()
        
        critical = sum(1 for i in self.issues if i.severity == "critical")
        high = sum(1 for i in self.issues if i.severity == "high")
        medium = sum(1 for i in self.issues if i.severity == "medium")
        
        health_score = 100
        health_score -= critical * 20
        health_score -= high * 10
        health_score -= medium * 5
        health_score = max(0, health_score)
        
        return {
            "is_windows": self.is_windows,
            "platform": platform.system(),
            "python_version": platform.python_version(),
            "health_score": health_score,
            "issues_count": {
                "critical": critical,
                "high": high,
                "medium": medium,
                "total": len(self.issues)
            },
            "issues": [i.to_dict() for i in self.issues],
            "can_auto_fix": sum(1 for i in self.issues if i.fix_command) > 0
        }


def get_windows_health() -> dict:
    """
    Convenience function to get Windows health report.
    
    Returns:
        Health report dictionary
    """
    detector = WindowsDetector()
    return detector.get_health_report()


if __name__ == "__main__":
    # CLI for testing
    import json
    detector = WindowsDetector()
    report = detector.get_health_report()
    print(json.dumps(report, indent=2))
