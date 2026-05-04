#!/usr/bin/env python3
"""
TARGET 2: MC1-MC5 coordination rule violations.
Analyzes git history for multi-cortex coordination gaps.
"""

import subprocess
import json
import logging
from collections import defaultdict
from datetime import datetime

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

def get_unmerged_branches():
    """Get branches not merged to main"""
    result = subprocess.run(
        ['git', 'branch', '--no-merged', 'main'],
        capture_output=True, text=True
    )
    branches = [b.strip() for b in result.stdout.split('\n') if b.strip() and not b.startswith('*')]
    return branches

def get_branch_info(branch):
    """Get commit count and files for a branch"""
    # Commit count
    count_result = subprocess.run(
        ['git', 'log', f'main..{branch}', '--oneline'],
        capture_output=True, text=True
    )
    count = len([l for l in count_result.stdout.strip().split('\n') if l])

    # Files modified
    files_result = subprocess.run(
        ['git', 'log', f'main..{branch}', '--name-only', '--pretty=format:'],
        capture_output=True, text=True
    )
    files = set(f for f in files_result.stdout.split('\n') if f.strip())

    # Last commit date
    date_result = subprocess.run(
        ['git', 'log', '-1', '--format=%ai', branch],
        capture_output=True, text=True
    )
    date_str = date_result.stdout.strip()

    return {
        'commits': count,
        'files': files,
        'last_commit': date_str,
    }

def extract_scope(branch_name):
    """Extract scope from branch name: feat/<scope>-DATE"""
    parts = branch_name.split('/')
    if len(parts) >= 2:
        scope_part = parts[1].rsplit('-', 1)[0]  # Remove date suffix
        return scope_part
    return 'unknown'

def analyze_mc_violations():
    """Analyze git history for MC1-MC5 violations"""
    log.info("=== TARGET 2: MC1-MC5 COORDINATION VIOLATIONS ===\n")

    branches = get_unmerged_branches()
    log.info(f"Found {len(branches)} unmerged branches\n")

    # Collect info
    branch_info = {}
    for branch in branches:
        info = get_branch_info(branch)
        branch_info[branch] = info

    # MC1: One branch = one cortex = one scope
    log.info("MC1 VIOLATIONS (overlapping branches on same scope):\n")
    scopes = defaultdict(list)
    for branch in branches:
        scope = extract_scope(branch)
        scopes[scope].append(branch)

    mc1_violations = {scope: brs for scope, brs in scopes.items() if len(brs) > 1}
    if mc1_violations:
        for scope, branches_list in sorted(mc1_violations.items()):
            log.info(f"  {scope}: {len(branches_list)} branches")
            for br in branches_list:
                info = branch_info[br]
                log.info(f"    - {br} ({info['commits']} commits, {len(info['files'])} files)")
            # Check file overlap
            all_files = [set() for _ in branches_list]
            for i, br in enumerate(branches_list):
                all_files[i] = branch_info[br]['files']
            overlap = set.intersection(*all_files) if all_files else set()
            if overlap:
                log.info(f"    CONFLICT: Overlapping files: {', '.join(sorted(list(overlap))[:3])}...\n")
    else:
        log.info("  None detected\n")

    # MC2: PR before new work
    log.info("\nMC2 VIOLATIONS (unmerged branches >1 day old):\n")
    mc2_violations = []
    for branch, info in branch_info.items():
        # Parse date: "2026-04-30 20:22:03 +0200"
        try:
            date_obj = datetime.fromisoformat(info['last_commit'].split(' ')[0])
            age_days = (datetime.fromisoformat('2026-05-02') - date_obj).days
            if age_days >= 1:
                mc2_violations.append((branch, age_days, info['commits']))
        except:
            pass

    if mc2_violations:
        mc2_violations.sort(key=lambda x: -x[1])
        for branch, age_days, commits in mc2_violations:
            log.info(f"  {branch}: {age_days} days old, {commits} commits\n")
    else:
        log.info("  None detected\n")

    # MC5: Atomic scope (branch scope matches files touched)
    log.info("\nMC5 VIOLATIONS (non-atomic scope — branch scope ≠ files touched):\n")
    mc5_violations = []
    for branch, info in branch_info.items():
        scope = extract_scope(branch)
        # Heuristic: if branch touches files from other domains, it's non-atomic
        file_domains = defaultdict(int)
        for f in info['files']:
            parts = f.split('/')
            if len(parts) > 1:
                domain = parts[0]  # cynic-python, cynic-kernel, .claude, etc.
                file_domains[domain] += 1

        # If scope is "wallet" but files touch hermes, kernel, etc. = non-atomic
        if len(file_domains) > 2:  # More than 2 domains = scope creep
            mc5_violations.append({
                'branch': branch,
                'declared_scope': scope,
                'domains_touched': dict(file_domains),
                'commits': info['commits'],
            })

    if mc5_violations:
        for v in mc5_violations:
            log.info(f"  {v['branch']}: declared={v['declared_scope']}, touched={list(v['domains_touched'].keys())}\n")
    else:
        log.info("  None detected\n")

    # Summary
    log.info("\n=== SUMMARY ===")
    log.info(f"MC1 (overlapping branches): {len(mc1_violations)}")
    log.info(f"MC2 (unmerged >1 day): {len(mc2_violations)}")
    log.info(f"MC5 (non-atomic scope): {len(mc5_violations)}\n")

if __name__ == '__main__':
    analyze_mc_violations()
