#!/usr/bin/env python3
"""
Stability monitoring for governance bot memory leak fix.
Monitors memory usage every 30 seconds for specified duration.
Simple version without unicode emojis (Windows compatibility).
"""

import asyncio
import os
import sys
import time
from datetime import datetime, timedelta
from pathlib import Path
import psutil
import json

# Configuration
MONITOR_DURATION_MINUTES = 30  # Run for 30 minutes
CHECK_INTERVAL_SECONDS = 30
PID_FILE = Path("governance_bot/bot_stability.pid")
LOG_FILE = Path("governance_bot/bot_stability_test.log")
METRICS_FILE = Path("governance_bot/stability_metrics.json")


class StabilityMonitor:
    """Monitor bot memory and health."""

    def __init__(self, pid: int):
        self.pid = pid
        self.process = psutil.Process(pid)
        self.metrics = []
        self.start_time = datetime.now()
        self.baseline_memory = None

    def get_memory_info(self) -> dict:
        """Get current memory info for the process."""
        try:
            mem_info = self.process.memory_info()
            memory_mb = mem_info.rss / 1024 / 1024

            return {
                "timestamp": datetime.now().isoformat(),
                "memory_mb": round(memory_mb, 2),
                "memory_percent": round(self.process.memory_percent(), 2),
                "pid": self.pid,
                "status": self.process.status(),
            }
        except psutil.NoSuchProcess:
            return None

    def check_logs(self) -> dict:
        """Check bot logs for errors."""
        if not LOG_FILE.exists():
            return {"error_count": 0, "warning_count": 0, "last_lines": []}

        with open(LOG_FILE) as f:
            lines = f.readlines()

        # Get last 20 lines
        last_lines = [line.strip() for line in lines[-20:]]

        # Count errors and warnings
        error_count = sum(1 for line in lines if "ERROR" in line)
        warning_count = sum(1 for line in lines if "WARNING" in line)
        attribute_errors = sum(
            1
            for line in lines
            if "AttributeError" in line or "has no attribute" in line
        )

        return {
            "error_count": error_count,
            "warning_count": warning_count,
            "attribute_errors": attribute_errors,
            "last_lines": last_lines[-10:],
        }

    async def monitor(self):
        """Run stability monitoring."""
        print("\n" + "=" * 70)
        print("GOVERNANCE BOT STABILITY TEST")
        print("=" * 70)
        print(f"Start time: {self.start_time}")
        print(f"Duration: {MONITOR_DURATION_MINUTES} minutes")
        print(f"Check interval: {CHECK_INTERVAL_SECONDS} seconds")
        print(f"PID: {self.pid}")
        print("=" * 70 + "\n")

        end_time = self.start_time + timedelta(minutes=MONITOR_DURATION_MINUTES)
        iteration = 0

        while datetime.now() < end_time:
            iteration += 1
            elapsed = (datetime.now() - self.start_time).total_seconds() / 60

            # Get memory info
            mem = self.get_memory_info()

            if mem is None:
                print(f"[{iteration:3d}] Process died! Aborting.")
                return False

            # Set baseline on first iteration
            if self.baseline_memory is None:
                self.baseline_memory = mem["memory_mb"]
                print(f"[BASELINE] Memory: {self.baseline_memory:.1f} MB")
                print()

            # Calculate change from baseline
            change_mb = mem["memory_mb"] - self.baseline_memory
            change_pct = (change_mb / self.baseline_memory) * 100

            # Format output
            memory_str = f"{mem['memory_mb']:6.1f} MB"
            change_str = f"{change_mb:+6.1f} MB ({change_pct:+6.1f}%)"
            status_str = f"[{elapsed:5.1f} min]"

            # Color code based on memory growth
            if change_pct > 50:
                indicator = "[CRITICAL]"
            elif change_pct > 20:
                indicator = "[GROWING]"
            else:
                indicator = "[STABLE]"

            print(f"{status_str} {memory_str} {change_str}  {indicator}")

            self.metrics.append(
                {
                    "iteration": iteration,
                    "elapsed_minutes": round(elapsed, 2),
                    **mem,
                    "change_from_baseline_mb": round(change_mb, 2),
                    "change_from_baseline_pct": round(change_pct, 2),
                }
            )

            await asyncio.sleep(CHECK_INTERVAL_SECONDS)

        return True

    def generate_report(self) -> str:
        """Generate stability test report."""
        if not self.metrics:
            return "No metrics collected"

        first_mem = self.metrics[0]["memory_mb"]
        last_mem = self.metrics[-1]["memory_mb"]
        max_mem = max(m["memory_mb"] for m in self.metrics)
        min_mem = min(m["memory_mb"] for m in self.metrics)
        avg_mem = sum(m["memory_mb"] for m in self.metrics) / len(self.metrics)

        # Get log info
        log_info = self.check_logs()

        # Build report
        report = f"""
{'='*70}
STABILITY TEST REPORT
{'='*70}

Test Duration: {MONITOR_DURATION_MINUTES} minutes
Checks: {len(self.metrics)} (every {CHECK_INTERVAL_SECONDS} seconds)
Start Time: {self.start_time}
End Time: {datetime.now()}

MEMORY METRICS
{'='*70}
Initial:        {first_mem:7.1f} MB
Final:          {last_mem:7.1f} MB
Minimum:        {min_mem:7.1f} MB
Maximum:        {max_mem:7.1f} MB
Average:        {avg_mem:7.1f} MB
Growth:         {last_mem - first_mem:+7.1f} MB ({((last_mem - first_mem) / first_mem * 100):+6.1f}%)

STABILITY ANALYSIS
{'='*70}
"""

        # Check stability
        max_growth = max(m["change_from_baseline_pct"] for m in self.metrics)
        if max_growth < 10:
            status = "[PASS] EXCELLENT - Memory perfectly stable"
        elif max_growth < 20:
            status = "[PASS] GOOD - Minor fluctuations, within acceptable range"
        elif max_growth < 50:
            status = "[WARN] WARNING - Memory growing, but not critical"
        else:
            status = "[FAIL] CRITICAL - Memory leak detected"

        report += f"{status}\n\nMax growth from baseline: {max_growth:+6.1f}%\n"

        # Log analysis
        report += f"""
ERROR ANALYSIS
{'='*70}
Total Errors:        {log_info['error_count']}
Total Warnings:      {log_info['warning_count']}
AttributeErrors:     {log_info['attribute_errors']}

"""

        if log_info["attribute_errors"] > 0:
            report += "[FAIL] CRITICAL: AttributeErrors detected - memory leak NOT fixed!\n"
        elif log_info["error_count"] == 0:
            report += "[PASS] GOOD: No errors logged\n"
        else:
            report += f"[WARN] WARNING: {log_info['error_count']} errors logged (check manually)\n"

        report += f"\nLast 10 log lines:\n"
        for line in log_info["last_lines"]:
            report += f"  {line}\n"

        report += f"\n{'='*70}\nCONCLUSION\n{'='*70}\n"

        if log_info["attribute_errors"] == 0 and max_growth < 20:
            report += "[PASS] Bot is STABLE. Memory leak is FIXED.\n"
            report += "       Ready for multi-instance validation and fine-tuning.\n"
        elif log_info["attribute_errors"] == 0 and max_growth < 50:
            report += "[WARN] Bot stable but monitor memory growth.\n"
        else:
            report += "[FAIL] Memory leak still present or errors detected.\n"

        report += f"{'='*70}\n"

        return report


async def main():
    """Run stability test."""
    # Check if PID file exists
    if not PID_FILE.exists():
        print("[ERROR] Bot not running! Start with: cd governance_bot && python bot.py")
        return 1

    try:
        with open(PID_FILE) as f:
            pid = int(f.read().strip())
    except (ValueError, IOError):
        print("[ERROR] Cannot read PID file")
        return 1

    # Create monitor and run
    monitor = StabilityMonitor(pid)

    try:
        success = await monitor.monitor()
    except KeyboardInterrupt:
        print("\n\n[INTERRUPT] Test interrupted by user")
    except Exception as e:
        print(f"\n\n[ERROR] Error during monitoring: {e}")
        return 1

    # Generate report
    print("\n" + "=" * 70)
    print("Generating report...")
    print("=" * 70)

    report = monitor.generate_report()
    print(report)

    # Save metrics to file
    with open(METRICS_FILE, "w") as f:
        json.dump(
            {
                "start_time": monitor.start_time.isoformat(),
                "end_time": datetime.now().isoformat(),
                "metrics": monitor.metrics,
            },
            f,
            indent=2,
        )
    print(f"\n[OK] Metrics saved to {METRICS_FILE}")

    # Save report
    report_file = Path("governance_bot/STABILITY_TEST_REPORT.txt")
    with open(report_file, "w") as f:
        f.write(report)
    print(f"[OK] Report saved to {report_file}")

    return 0


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
