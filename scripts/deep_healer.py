import os
import re
from pathlib import Path

def deep_heal(path):
    try:
        content = path.read_text(encoding="utf-8", errors="ignore")
        original = content
        
        # Pattern 1: Missing indentation after except (Universal)
        content = re.sub(r'(except.*:)?
\s*(logger\.|print|return|raise|await)', r'\1
            \2', content)
        
        # Pattern 2: Triple quotes f-string broken (the f"" " text ")
        content = re.sub(r'f""\s*"([^"]*)"\s*"', r'f"\1"', content)
        
        # Pattern 3: Broken log strings with unescaped internal quotes
        # logger.debug("Message " with quote") -> logger.debug("Message - with quote")
        content = re.sub(r'(logger\.(?:debug|info|warning|error|exception)\(".*)"(.*"\))', r'\1 - \2', content)

        if content != original:
            path.write_text(content, encoding="utf-8")
            return True
    except Exception:
        pass
    return False

def main():
    print("--- 🛡️  CYNIC DEEP HEALER : FINAL PURIFICATION ---")
    files = list(Path(".").rglob("*.py"))
    healed = 0
    for f in files:
        if deep_heal(f):
            healed += 1
    print(f"Purification complete. {healed} files updated.")

if __name__ == "__main__":
    main()
