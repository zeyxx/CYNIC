#!/usr/bin/env python3
"""
Phase 1: Extract strategic knowledge from Claude Code session logs.

Goal: Parse all session JSONL files, extract decision points, and produce
a dataset that Askesis can consume and reflect on.

This is the data ingestion layer for LAYER 1 (PERCEPTION) of the
data-centric organism.
"""

import json
import sys
from pathlib import Path
from collections import defaultdict, Counter
from datetime import datetime
from dataclasses import dataclass, field, asdict
from typing import List, Dict, Optional


@dataclass
class LogEntry:
    """Structured representation of a Claude Code turn"""
    timestamp: str
    session_id: str
    turn: int
    user_type: str  # "user" or "assistant"
    message_length: int
    has_tool_result: bool
    message_preview: str  # First 100 chars
    cwd: Optional[str] = None
    git_branch: Optional[str] = None


@dataclass
class SessionSummary:
    """Aggregate statistics for one session"""
    session_id: str
    title: str
    turn_count: int
    user_turns: int
    assistant_turns: int
    avg_message_length: float
    has_tool_results: bool
    date_range: tuple  # (start, end)
    dominant_branch: Optional[str]
    intent: str  # inferred from turns


class SessionAnalyzer:
    """Parse and analyze Claude Code session logs"""

    def __init__(self, project_dir: Path):
        self.project_dir = Path(project_dir)
        self.sessions: Dict[str, List[LogEntry]] = defaultdict(list)
        self.summaries: List[SessionSummary] = []

    def load_sessions(self) -> None:
        """Read all .jsonl session files from ~/.claude/projects"""
        session_files = list(self.project_dir.glob("*.jsonl"))
        print(f"Found {len(session_files)} session files", file=sys.stderr)

        for session_file in session_files:
            session_id = session_file.stem

            with open(session_file, 'r') as f:
                for line_num, line in enumerate(f, 1):
                    try:
                        obj = json.loads(line)

                        # Skip metadata entries
                        if obj.get('type') in ['custom-title', 'agent-name']:
                            continue

                        # Parse actual message entries
                        if obj.get('type') in ['user', 'assistant']:
                            msg = obj.get('message', '')
                            msg_str = msg if isinstance(msg, str) else str(msg)
                            entry = LogEntry(
                                timestamp=obj.get('timestamp', ''),
                                session_id=session_id,
                                turn=line_num,
                                user_type=obj.get('type'),
                                message_length=len(msg_str),
                                has_tool_result=bool(obj.get('toolUseResult')),
                                message_preview=msg_str[:100],
                                cwd=obj.get('cwd'),
                                git_branch=obj.get('gitBranch'),
                            )
                            self.sessions[session_id].append(entry)
                    except json.JSONDecodeError:
                        continue

    def analyze(self) -> None:
        """Compute summaries for each session"""
        for session_id, entries in self.sessions.items():
            if not entries:
                continue

            user_turns = [e for e in entries if e.user_type == 'user']
            assistant_turns = [e for e in entries if e.user_type == 'assistant']

            timestamps = [e.timestamp for e in entries if e.timestamp]
            if timestamps:
                try:
                    dates = [datetime.fromisoformat(ts.replace('Z', '+00:00')) for ts in timestamps]
                    date_range = (min(dates), max(dates))
                except:
                    date_range = (None, None)
            else:
                date_range = (None, None)

            branches = [e.git_branch for e in entries if e.git_branch]
            dominant_branch = Counter(branches).most_common(1)[0][0] if branches else None

            avg_length = sum(e.message_length for e in entries) / len(entries) if entries else 0

            # Infer intent from turn count and branches touched
            if len(user_turns) < 5:
                intent = "quick_check"
            elif "feat/" in (dominant_branch or ""):
                intent = "feature_development"
            elif "fix/" in (dominant_branch or ""):
                intent = "bug_fix"
            elif any("refactor" in e.message_preview.lower() for e in user_turns):
                intent = "refactoring"
            else:
                intent = "mixed_work"

            summary = SessionSummary(
                session_id=session_id,
                title=f"Session {session_id[:8]}",
                turn_count=len(entries),
                user_turns=len(user_turns),
                assistant_turns=len(assistant_turns),
                avg_message_length=avg_length,
                has_tool_results=any(e.has_tool_result for e in entries),
                date_range=date_range,
                dominant_branch=dominant_branch,
                intent=intent,
            )
            self.summaries.append(summary)

    def extract_patterns(self) -> Dict:
        """Find recurring patterns across sessions"""
        patterns = {
            "total_sessions": len(self.summaries),
            "total_turns": sum(s.turn_count for s in self.summaries),
            "avg_turns_per_session": sum(s.turn_count for s in self.summaries) / len(self.summaries) if self.summaries else 0,
            "feature_development_sessions": sum(1 for s in self.summaries if s.intent == "feature_development"),
            "bug_fix_sessions": sum(1 for s in self.summaries if s.intent == "bug_fix"),
            "refactoring_sessions": sum(1 for s in self.summaries if s.intent == "refactoring"),
            "quick_check_sessions": sum(1 for s in self.summaries if s.intent == "quick_check"),
            "sessions_with_tools": sum(1 for s in self.summaries if s.has_tool_results),
            "avg_message_length": sum(s.avg_message_length for s in self.summaries) / len(self.summaries) if self.summaries else 0,
        }

        # Branch activity
        branches = Counter()
        for s in self.summaries:
            if s.dominant_branch:
                branches[s.dominant_branch] += 1
        patterns["active_branches"] = dict(branches.most_common(5))

        return patterns

    def output_datasets(self, output_dir: Path) -> None:
        """Write structured datasets for Askesis"""
        output_dir.mkdir(parents=True, exist_ok=True)

        # Dataset 1: Raw session summaries
        with open(output_dir / "session_summaries.jsonl", 'w') as f:
            for summary in self.summaries:
                f.write(json.dumps(asdict(summary), default=str) + '\n')

        # Dataset 2: Patterns
        patterns = self.extract_patterns()
        with open(output_dir / "session_patterns.json", 'w') as f:
            json.dump(patterns, f, indent=2, default=str)

        # Dataset 3: Intent distribution (for reflection)
        intents = Counter(s.intent for s in self.summaries)
        with open(output_dir / "session_intents.json", 'w') as f:
            json.dump(dict(intents), f, indent=2)

        print(f"Wrote datasets to {output_dir}", file=sys.stderr)


def main():
    # Find project root
    project_dir = Path.home() / ".claude" / "projects" / "-home-user-Bureau-CYNIC"

    if not project_dir.exists():
        print(f"Project dir not found: {project_dir}", file=sys.stderr)
        sys.exit(1)

    analyzer = SessionAnalyzer(project_dir)

    print("PHASE 1: PERCEPTION", file=sys.stderr)
    print("Loading sessions...", file=sys.stderr)
    analyzer.load_sessions()

    print(f"Loaded {len(analyzer.sessions)} sessions", file=sys.stderr)

    print("Analyzing...", file=sys.stderr)
    analyzer.analyze()

    print("Extracting patterns...", file=sys.stderr)
    patterns = analyzer.extract_patterns()

    # Output
    output_dir = Path("/home/user/Bureau/CYNIC/.askesis/datasets")
    analyzer.output_datasets(output_dir)

    # Print summary
    print("\n=== SESSION ANALYSIS SUMMARY ===", file=sys.stderr)
    print(f"Total sessions: {patterns['total_sessions']}", file=sys.stderr)
    print(f"Total turns: {patterns['total_turns']}", file=sys.stderr)
    print(f"Avg turns/session: {patterns['avg_turns_per_session']:.1f}", file=sys.stderr)
    print(f"Sessions with tools: {patterns['sessions_with_tools']}", file=sys.stderr)
    print(f"Intent distribution:", file=sys.stderr)
    print(f"  - Feature development: {patterns['feature_development_sessions']}", file=sys.stderr)
    print(f"  - Bug fixes: {patterns['bug_fix_sessions']}", file=sys.stderr)
    print(f"  - Refactoring: {patterns['refactoring_sessions']}", file=sys.stderr)
    print(f"  - Quick checks: {patterns['quick_check_sessions']}", file=sys.stderr)


if __name__ == '__main__':
    main()
