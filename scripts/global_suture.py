import os
import re

MAPPINGS = [
    (r'cynic\.brain\b', 'cynic.kernel.organism.brain'),
    (r'cynic\.perception\b', 'cynic.kernel.organism.perception'),
    (r'cynic\.metabolism\b', 'cynic.kernel.organism.metabolism'),
]

EXCLUDE_DIRS = {'.git', '__pycache__', '.pytest_cache', '.mypy_cache'}

def suture_file(file_path):
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = content
        changes = 0
        for pattern, replacement in MAPPINGS:
            updated = re.sub(pattern, replacement, new_content)
            if updated != new_content:
                new_content = updated
                changes += 1
        
        if changes > 0:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)
            print(f"✅ Sutured: {file_path} ({changes} patterns updated)")
            return True
    except Exception as e:
        print(f"❌ Failed to suture {file_path}: {e}")
    return False

def run_global_suture():
    count = 0
    for root, dirs, files in os.walk('.'):
        dirs[:] = [d for d in dirs if d not in EXCLUDE_DIRS]
        for file in files:
            if file.endswith(('.py', '.md', '.toml', '.yml', '.yaml', '.sh')):
                if suture_file(os.path.join(root, file)):
                    count += 1
    print(f"\n🎯 Suture complete. {count} files updated.")

if __name__ == "__main__":
    run_global_suture()
