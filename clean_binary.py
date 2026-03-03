import os

def clean_file(filepath):
    try:
        with open(filepath, 'rb') as f:
            raw = f.read()
        
        # Simple byte filter: keep only ASCII (0-127)
        clean = bytes([b for b in raw if b < 128])
        
        if clean != raw:
            with open(filepath, 'wb') as f:
                f.write(clean)
            return True
    except Exception as e:
        print(f"Error: {filepath} -> {e}")
    return False

count = 0
for root, dirs, files in os.walk('cynic'):
    for file in files:
        if file.endswith('.py'):
            if clean_file(os.path.join(root, file)):
                count += 1
print(f"Cleaned {count} files.")
