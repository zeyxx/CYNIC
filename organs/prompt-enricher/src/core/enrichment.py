"""Prompt enrichment domain logic."""

from __future__ import annotations


def enrich_prompt(raw_text: str) -> str:
    """Apply the current deterministic prompt scaffold."""

    return f"""<thought>
Analyze this problem step by step before writing code.
</thought>
<constraints>
- Security: no SQL injection
- Privacy: minimize data collection
</constraints>

[USER TASK]
{raw_text}
"""
