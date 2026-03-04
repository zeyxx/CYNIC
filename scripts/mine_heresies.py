"""
CYNIC Heresy Miner - Deep Forensics of LLM Generation Failures.
Respects Data Engineer & SRE Lenses.

Extracts the exact context of every E999 (SyntaxError) in the codebase.
This transforms 'bugs' into a structured dataset for the MCTS Scientist to learn from.
"""
import subprocess
import json
from pathlib import Path
from collections import defaultdict

def run_ruff():
    """Runs ruff to get the raw list of E999 errors."""
    print("Executing Ruff scan...")
    result = subprocess.run(
        ["ruff", "check", ".", "--select", "E999"], 
        capture_output=True, 
        text=True
    )
    return result.stdout

def mine_context(filepath: str, line_num: int, context_size: int = 3) -> dict:
    """Extracts the code surrounding the syntax error."""
    try:
        path = Path(filepath)
        if not path.exists():
            return {"error": "File not found"}
            
        with open(path, 'r', encoding='utf-8', errors='ignore') as f:
            lines = f.readlines()
            
        start_idx = max(0, line_num - 1 - context_size)
        end_idx = min(len(lines), line_num + context_size)
        
        context_lines = []
        for i in range(start_idx, end_idx):
            prefix = ">> " if i == line_num - 1 else "   "
            # Strip trailing newlines for cleaner JSON
            context_lines.append(f"{i+1:4d} | {prefix}{lines[i].rstrip()}")
            
        return {
            "snippet": "\n".join(context_lines),
            "raw_line": lines[line_num - 1].strip() if line_num <= len(lines) else ""
        }
    except Exception as e:
        return {"error": str(e)}

def classify_heresy(error_msg: str, raw_line: str) -> str:
    """Basic categorization of the LLM hallucination."""
    msg = error_msg.lower()
    if "indent" in msg:
        return "INDENTATION_FRACTURE"
    elif "quote" in msg or 'f"' in raw_line or "f'" in raw_line:
        return "BROKEN_F_STRING"
    elif "expected ','" in msg:
        return "MISSING_COMMA_DICT_LIST"
    elif "expected ')'" in msg or "expected ']'" in msg:
        return "UNCLOSED_BRACKET"
    return "UNKNOWN_SYNTAX_MUTATION"

def main():
    print("--- ⛏️ CYNIC FORENSICS : MINING THE ASYMPTOTE ---")
    
    stdout = run_ruff()
    output_dir = Path("audit")
    output_dir.mkdir(exist_ok=True)
    out_file = output_dir / "heresy_mine.jsonl"
    
    stats = defaultdict(int)
    total_mined = 0
    
    with open(out_file, 'w', encoding='utf-8') as f:
        for line in stdout.splitlines():
            # Example ruff output:
            # cynic\kernel\organism\brain\dialogue\llm_bridge.py:143:1: Unexpected indentation
            # OR
            # error: Failed to parse path/to/file.py:10:5: Expected ...
            
            if "E999" not in line and "Failed to parse" not in line:
                continue
                
            parts = line.split(":")
            if len(parts) >= 4:
                # Handle "error: Failed to parse FILE:LINE:COL: MSG"
                if parts[0] == "error" and "Failed to parse" in parts[0]:
                    filepath = parts[0].split("Failed to parse ")[-1].strip()
                    try:
                        line_num = int(parts[1])
                        error_msg = ":".join(parts[3:]).strip()
                    except ValueError:
                        continue
                else:
                    # Standard format FILE:LINE:COL: MSG
                    filepath = parts[0].strip()
                    # Handle windows paths C:\...
                    if len(filepath) == 1 and parts[0].isalpha():
                        filepath = parts[0] + ":" + parts[1]
                        line_offset = 1
                    else:
                        line_offset = 0
                        
                    try:
                        line_num = int(parts[1 + line_offset])
                        error_msg = ":".join(parts[3 + line_offset:]).strip()
                    except ValueError:
                        continue
                
                context = mine_context(filepath, line_num)
                raw_line = context.get("raw_line", "")
                
                category = classify_heresy(error_msg, raw_line)
                stats[category] += 1
                
                record = {
                    "file": filepath,
                    "line": line_num,
                    "error": error_msg,
                    "category": category,
                    "context": context.get("snippet", "")
                }
                
                f.write(json.dumps(record) + "\n")
                total_mined += 1

    print(f"\n✅ Mining complete. Extracted {total_mined} veins of failure.")
    print(f"Data sealed in: {out_file}")
    print("\n📊 HERESY DISTRIBUTION:")
    for cat, count in sorted(stats.items(), key=lambda x: x[1], reverse=True):
        print(f"  - {cat}: {count}")

if __name__ == "__main__":
    main()
