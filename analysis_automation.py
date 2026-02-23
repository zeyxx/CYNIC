#!/usr/bin/env python3
"""
CYNIC Empirical Archaeology - Automation Phase
Extract patterns, MCTS, Q-learning, emergence from old CYNIC
"""

import os
import json
import re
from pathlib import Path
from collections import defaultdict
from datetime import datetime

# Configuration
CYNIC_ROOT = Path("/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC")
OUTPUT_DIR = Path("/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC-clean/cynic/docs/analysis")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Keywords to search for
KEYWORDS = {
    "MCTS": ["mcts", "monte carlo", "tree search", "ucb", "rollout"],
    "Q_LEARNING": ["q-learning", "q-table", "arbre_q", "temporal difference", "bellman"],
    "EMERGENCE": ["emergence", "emergent", "threshold", "singularity", "phase transition", "criticality"],
    "PATTERNS": ["pattern", "crystalliz", "consolidat", "memory", "persistence"],
    "CONSCIOUSNESS": ["consciousness", "organism", "alive", "aware", "meta-cognit"],
    "AXIOMS": ["axiom", "phi", "fidelity", "verify", "culture", "burn"],
    "LEARNING": ["learning", "adapt", "improve", "feedback", "loop"],
    "DISTRIBUTED": ["distributed", "federated", "consensus", "node", "collective"],
}

def scan_python_files():
    """Scan all Python files for keywords"""
    results = defaultdict(list)

    for py_file in CYNIC_ROOT.rglob("*.py"):
        if ".venv" in str(py_file) or "__pycache__" in str(py_file):
            continue

        try:
            with open(py_file, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()

            for category, keywords in KEYWORDS.items():
                for keyword in keywords:
                    if keyword.lower() in content.lower():
                        # Find context (lines containing keyword)
                        lines = content.split('\n')
                        for i, line in enumerate(lines):
                            if keyword.lower() in line.lower():
                                results[category].append({
                                    'file': str(py_file.relative_to(CYNIC_ROOT)),
                                    'line': i + 1,
                                    'context': line.strip()[:100]
                                })
        except Exception as e:
            pass

    return results

def scan_markdown_files():
    """Scan markdown documentation for architectural insights"""
    results = {}

    for md_file in CYNIC_ROOT.rglob("*.md"):
        if ".venv" in str(md_file):
            continue

        try:
            with open(md_file, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()

            # Extract key sections
            sections = re.findall(r'^#+\s+(.+)$', content, re.MULTILINE)

            if any(kw in content.lower() for category in KEYWORDS.values() for kw in category):
                results[str(md_file.relative_to(CYNIC_ROOT))] = {
                    'sections': sections[:10],
                    'length': len(content),
                    'has_code': '```' in content
                }
        except Exception as e:
            pass

    return results

def analyze_git_history():
    """Analyze git commits for pattern evolution"""
    import subprocess

    os.chdir(CYNIC_ROOT)

    try:
        # Get last 50 commits
        result = subprocess.run(
            ['git', 'log', '--oneline', '-50'],
            capture_output=True,
            text=True
        )

        commits = result.stdout.strip().split('\n')

        # Categorize commits
        categories = defaultdict(list)
        for commit in commits:
            if 'MCTS' in commit or 'tree' in commit.lower():
                categories['MCTS_related'].append(commit)
            elif 'Q-' in commit or 'learning' in commit.lower():
                categories['Learning_related'].append(commit)
            elif 'emergency' in commit.lower() or 'emerge' in commit.lower():
                categories['Emergence_related'].append(commit)
            elif 'organism' in commit.lower() or 'consciousness' in commit.lower():
                categories['Consciousness_related'].append(commit)

        return categories
    except Exception as e:
        return {'error': str(e)}

def extract_json_structure():
    """Extract meaningful JSON files (configs, data)"""
    results = []

    for json_file in CYNIC_ROOT.rglob("*.json"):
        if ".venv" in str(json_file) or ".hypothesis" in str(json_file):
            continue

        try:
            with open(json_file, 'r') as f:
                data = json.load(f)

            # Look for MCTS, Q-learning, patterns
            json_str = json.dumps(data, indent=2)

            if any(kw in json_str.lower() for category in KEYWORDS.values() for kw in category):
                results.append({
                    'file': str(json_file.relative_to(CYNIC_ROOT)),
                    'size': len(json_str),
                    'keys': list(data.keys())[:5] if isinstance(data, dict) else None
                })
        except Exception as e:
            pass

    return results

def main():
    """Execute all automation scans"""

    print("[AUTOMATION] Starting empirical archaeology scan...")
    print(f"[AUTOMATION] Root: {CYNIC_ROOT}")

    # 1. Python scanning
    print("\n[SCAN 1] Analyzing Python files for keywords...")
    py_results = scan_python_files()

    # 2. Markdown scanning
    print("[SCAN 2] Analyzing Markdown documentation...")
    md_results = scan_markdown_files()

    # 3. Git history
    print("[SCAN 3] Analyzing git commit history...")
    git_results = analyze_git_history()

    # 4. JSON structures
    print("[SCAN 4] Extracting JSON structures...")
    json_results = extract_json_structure()

    # 5. Compile report
    report = {
        'timestamp': datetime.now().isoformat(),
        'scan_root': str(CYNIC_ROOT),
        'python_findings': {k: len(v) for k, v in py_results.items()},
        'markdown_documents': len(md_results),
        'git_categories': {k: len(v) for k, v in git_results.items()},
        'json_files_relevant': len(json_results),
        'key_python_keywords': py_results,
        'markdown_structure': md_results,
        'git_history_summary': git_results,
        'json_structures': json_results
    }

    # Write report
    report_file = OUTPUT_DIR / "automation_scan_report.json"
    with open(report_file, 'w') as f:
        json.dump(report, f, indent=2)

    print(f"\n[AUTOMATION] Report saved: {report_file}")

    # Print summary
    print("\n=== AUTOMATION SCAN SUMMARY ===")
    print(f"Python keyword findings:")
    for category, count in report['python_findings'].items():
        print(f"  {category}: {count} occurrences")

    print(f"\nMarkdown documents found: {report['markdown_documents']}")
    print(f"\nGit commit categories:")
    for category, count in report['git_categories'].items():
        print(f"  {category}: {count}")

    print(f"\nJSON files with relevant patterns: {report['json_files_relevant']}")
    print("\n=== NEXT STEP: Manual analysis ===")

if __name__ == '__main__':
    main()
