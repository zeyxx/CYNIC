import os
import re
from pathlib import Path

def suture_file(path):
    with open(path, 'r', encoding='utf-8', errors='ignore') as f:
        lines = f.readlines()
    
    new_lines = []
    modified = False
    
    for i in range(len(lines)):
        new_lines.append(lines[i])
        # Match 'except Exception as ...:' or 'except:'
        if re.search(r'^\s*except.*:', lines[i]):
            # Check the next non-empty line
            j = i + 1
            while j < len(lines) and not lines[j].strip():
                j += 1
            
            if j < len(lines):
                current_indent = len(lines[i]) - len(lines[i].lstrip())
                next_line_indent = len(lines[j]) - len(lines[j].lstrip())
                
                # If the next line is a logger/print and NOT indented relative to except
                if (re.search(r'^\s*(logger|print|return|raise|await)', lines[j]) and 
                    next_line_indent <= current_indent):
                    
                    # Store indices to fix later (we can't easily fix in this loop)
                    # But for simplicity, let's just mark the file as needing fix
                    modified = True
                    # Simple fix: add 4 spaces to the next line
                    lines[j] = "    " + lines[j]
                    print(f"  [FIXED INDENT] at line {j+1} in {path}")

    if modified:
        with open(path, 'w', encoding='utf-8') as f:
            f.writelines(lines)
        return True
    return False

def main():
    print("--- 🛡️  CYNIC UNIVERSAL SUTURE : RESTORING SPINAL FLOW ---")
    cynic_dir = Path("cynic")
    files = list(cynic_dir.rglob("*.py"))
    
    count = 0
    for f in files:
        if suture_file(str(f)):
            count += 1
            
    print(f"\nSUTURE COMPLETE. {count} files restored.")

if __name__ == "__main__":
    main()
