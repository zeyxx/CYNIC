import os
import string
from pathlib import Path

def hammer_file(path):
    try:
        # Read with ignore to strip binary garbage
        content = path.read_text(encoding="utf-8", errors="ignore")
        
        # Standardize line endings to Unix (LF)
        content = content.replace("
", "
")
        
        # Fix the most common LLM-generated string break
        # (physical newline inside a quoted string)
        lines = content.split("
")
        fixed_lines = []
        for line in lines:
            # If line ends with a quote but was clearly cut
            if line.count('"') % 2 != 0 and not line.strip().endswith('"""'):
                line = line + '"' # Close it (brute force)
            fixed_lines.append(line)
        
        path.write_text("
".join(fixed_lines), encoding="utf-8")
        return True
    except Exception as e:
        print(f"Error hammering {path}: {e}")
        return False

def main():
    print("--- 🔨 CYNIC ASCII HAMMER : STABILIZING PARSER ---")
    files = list(Path("cynic").rglob("*.py")) + list(Path("scripts").rglob("*.py"))
    count = 0
    for f in files:
        if hammer_file(f):
            count += 1
    print(f"Hammered {count} files.")

if __name__ == "__main__":
    main()
