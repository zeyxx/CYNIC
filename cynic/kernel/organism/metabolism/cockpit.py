"""
CYNIC Cockpit - Real-time Night Cycle Dashboard.
Generates a human-readable summary of the autonomous emergence.
"""
from datetime import datetime
from pathlib import Path

class NightCockpit:
    def __init__(self, report_path: str = "NIGHT_REPORT.md"):
        self.path = Path(report_path)
        self._init_report()

    def _init_report(self):
        with open(self.path, "w", encoding="utf-8") as f:
            f.write(f"# 🌀 CYNIC NIGHT ONE - EMERGENCE COCKPIT\n")
            f.write(f"**Start Time:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            f.write(f"## 📊 Current Status\n- **Status:** 🟢 RUNNING\n- **Missions Completed:** 0\n- **Total Learning Shifts:** 0\n\n")
            f.write(f"## 🛠️ Mission Log\n| Time | Target | Axiom | Result | Model |\n| :--- | :--- | :--- | :--- | :--- |\n")

    def update_mission(self, target: str, axiom: str, result: str, muscle: str, success: bool):
        status_emoji = "✅" if success else "❌"
        time_str = datetime.now().strftime("%H:%M:%S")
        with open(self.path, "a", encoding="utf-8") as f:
            f.write(f"| {time_str} | `{target}` | `{axiom}` | {status_emoji} {result} | `{muscle}` |\n")

    def update_summary(self, missions_count: int, learning_shifts: int, status: str = "RUNNING"):
        content = self.path.read_text(encoding="utf-8")
        # Simple replacement logic for the header stats
        lines = content.splitlines()
        for i, line in enumerate(lines):
            if "Status:" in line: lines[i] = f"- **Status:** {status}"
            if "Missions Completed:" in line: lines[i] = f"- **Missions Completed:** {missions_count}"
            if "Total Learning Shifts:" in line: lines[i] = f"- **Total Learning Shifts:** {learning_shifts}"
        self.path.write_text("\n".join(lines), encoding="utf-8")
