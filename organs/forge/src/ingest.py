#!/usr/bin/env python3
import json
import subprocess
import sys
from pathlib import Path
from datetime import datetime

def ingest():
    output_dir = Path.home() / ".cynic" / "organs" / "hermes" / "github"
    output_dir.mkdir(parents=True, exist_ok=True)
    output_file = output_dir / "stars.jsonl"
    
    print("Fetching live GitHub stars via 'gh' CLI...")
    try:
        result = subprocess.run(
            ["gh", "api", "user/starred", "--paginate"],
            capture_output=True,
            text=True,
            check=True
        )
    except FileNotFoundError:
        print("Error: 'gh' CLI not found on PATH. Make sure GitHub CLI is installed.", file=sys.stderr)
        sys.exit(1)
    except subprocess.CalledProcessError as e:
        print(f"Error calling gh CLI: {e.stderr}", file=sys.stderr)
        sys.exit(1)

    text = result.stdout.strip()
    if not text:
        print("No starred repositories found or empty response.", file=sys.stderr)
        return

    # Decode concatenated JSON arrays output by gh api --paginate
    decoder = json.JSONDecoder()
    pos = 0
    raw_stars = []
    while pos < len(text):
        while pos < len(text) and text[pos].isspace():
            pos += 1
        if pos >= len(text):
            break
        obj, next_pos = decoder.raw_decode(text, pos)
        if isinstance(obj, list):
            raw_stars.extend(obj)
        else:
            raw_stars.append(obj)
        pos = next_pos

    stars = []
    with open(output_file, 'w') as f:
        for item in raw_stars:
            star = {
                "name": item.get("full_name"),
                "description": item.get("description"),
                "language": item.get("language"),
                "ingested_at": datetime.now().isoformat()
            }
            f.write(json.dumps(star) + '\n')
            stars.append(star)

    print(f"Ingested {len(stars)} stars to {output_file}")

if __name__ == "__main__":
    ingest()

