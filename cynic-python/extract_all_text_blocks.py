#!/usr/bin/env python3
"""
Extract ALL text blocks from sessions (no keyword filtering).
Will be scored by Qwen 9B for decision-ness via semantic analysis.
"""

import json
from pathlib import Path
from dataclasses import dataclass
import logging

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

@dataclass
class TextBlock:
    """Raw text block from session"""
    session_id: str
    timestamp: str
    cortex_type: str
    domain: str
    block_index: int
    text: str
    char_count: int

def extract_all_blocks(session_path: Path) -> list[TextBlock]:
    """Extract all text blocks from a session JSONL file"""
    blocks = []

    try:
        with open(session_path) as f:
            for line_idx, line in enumerate(f):
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue

                # Extract text from content array
                content_array = entry.get('content', [])
                if not content_array or not isinstance(content_array, list):
                    continue

                timestamp = entry.get('timestamp', 'unknown')
                cortex_type = entry.get('type', 'unknown')
                domain = entry.get('domain', 'general')
                session_id = session_path.stem

                for block_idx, content_item in enumerate(content_array):
                    if isinstance(content_item, dict) and 'text' in content_item:
                        text = content_item['text'].strip()
                        if len(text) > 50:  # Filter out noise/whitespace
                            block = TextBlock(
                                session_id=session_id,
                                timestamp=timestamp,
                                cortex_type=cortex_type,
                                domain=domain,
                                block_index=block_idx,
                                text=text,
                                char_count=len(text),
                            )
                            blocks.append(block)

    except Exception as e:
        log.warning(f"Error processing {session_path}: {e}")

    return blocks

def main():
    session_dir = Path.home() / '.gemini' / 'tmp' / 'cynic' / 'chats'

    # Use same 9 sessions as before
    session_names = [
        'session-2026-04-28T12-58-b0eba480.jsonl',
        'session-2026-04-25T12-52-83d14cea.jsonl',
        'session-2026-04-25T14-35-cbbd1ad7.jsonl',
        'session-2026-04-26T12-15-119d0d35.jsonl',
        'session-2026-04-26T14-03-ad8603c3.jsonl',
        'session-2026-04-24T13-43-2986cfe1.jsonl',
        'session-2026-04-25T10-44-03d195b3.jsonl',
        'session-2026-04-29T10-13-44499c90.jsonl',
        'session-2026-04-30T17-15-05cc8234.jsonl',
    ]

    all_blocks = []
    for name in session_names:
        path = session_dir / name
        if path.exists():
            blocks = extract_all_blocks(path)
            all_blocks.extend(blocks)
            log.info(f"  {name}: {len(blocks)} text blocks")
        else:
            log.warning(f"  {name}: not found")

    log.info(f"\n✓ Extracted {len(all_blocks)} total text blocks\n")

    # Save to JSONL (better for semantic scoring)
    output_file = Path('/home/user/Bureau/CYNIC/cynic-python/text_blocks_raw.jsonl')
    with open(output_file, 'w') as f:
        for block in all_blocks:
            line = {
                'session_id': block.session_id,
                'timestamp': block.timestamp,
                'cortex_type': block.cortex_type,
                'domain': block.domain,
                'block_index': block.block_index,
                'text': block.text,
                'char_count': block.char_count,
            }
            f.write(json.dumps(line) + '\n')

    log.info(f"✓ Saved to {output_file}")
    log.info(f"\nNext: Score with Qwen 9B for decision-ness:")
    log.info(f"  python3 cynic-python/score_decisions_qwen.py")

if __name__ == '__main__':
    main()
