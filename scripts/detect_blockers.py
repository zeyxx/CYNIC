#!/usr/bin/env python3
"""
Detect blocked sessions and suggest actions to unblock them.
Part of the "Ralph Loop" auto-continue system.
"""

import subprocess
import sys

# Ensure UTF-8 output on all platforms
if sys.stdout.encoding and 'utf' not in sys.stdout.encoding.lower():
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

def get_active_prs():
    """Get all open PRs for this repo."""
    try:
        # Note: In real implementation, use GitHub API
        # For now, read from git state
        result = subprocess.run(
            ['git', 'branch', '-r', '--format=%(refname:short)'],
            capture_output=True,
            text=True
        )
        branches = [b.replace('origin/', '') for b in result.stdout.strip().split('\n')
                   if b and 'main' not in b]
        return branches
    except:
        return []

def detect_blockers():
    """Find sessions that are blocked and suggest unblocking actions."""
    prs = get_active_prs()
    blockers = []

    for pr_branch in prs:
        # Check if this branch is behind main
        try:
            result = subprocess.run(
                ['git', 'merge-base', '--is-ancestor', 'origin/main', f'origin/{pr_branch}'],
                capture_output=True
            )
            if result.returncode != 0:
                # PR branch is behind main, might be blocked
                result = subprocess.run(
                    ['git', 'log', f'origin/main..origin/{pr_branch}', '--oneline'],
                    capture_output=True,
                    text=True
                )
                commits_ahead = len(result.stdout.strip().split('\n')) if result.stdout.strip() else 0

                if commits_ahead > 0:
                    blockers.append({
                        'branch': pr_branch,
                        'issue': f'{commits_ahead} commits ahead of main (might be blocked)',
                        'action': 'Pull main: git rebase origin/main',
                    })
        except:
            pass

    return blockers

def suggest_unblock_actions(blockers):
    """Suggest actions to unblock stalled sessions."""
    if not blockers:
        print("✅ No blockers detected")
        return

    print(f"⚠️  Found {len(blockers)} potential blockers:\n")
    for blocker in blockers:
        print(f"  📌 {blocker['branch']}")
        print(f"     Issue: {blocker['issue']}")
        print(f"     Action: {blocker['action']}")
        print()

if __name__ == '__main__':
    blockers = detect_blockers()
    suggest_unblock_actions(blockers)
