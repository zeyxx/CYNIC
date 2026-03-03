import os

def clean_file(filepath):
    try:
        with open(filepath, 'rb') as f:
            content = f.read()
        
        # Target the exact byte sequence discovered: â€"
        # It seems to be \xc3\xa2 \xe2\x82\xac followed by " or \xe2\x80\x9c
        
        targets = [
            b'\xc3\xa2\xe2\x82\xac"', # The one found in LOD 0
            b'\xc3\xa2\xe2\x82\xac\xe2\x80\x9c', # Variant
            b'\xc3\xa2\xe2\x82\xac\xe2\x84\xa2', # Another common one
            b'\xe2\x80\x94', # em dash
            b'\xe2\x80\x93', # en dash
        ]
        
        new_content = content
        for t in targets:
            new_content = new_content.replace(t, b'-')
            
        if new_content != content:
            with open(filepath, 'wb') as f:
                f.write(new_content)
            return True
    except Exception as e:
        print(f"Error cleaning {filepath}: {e}")
    return False

count = 0
for root, dirs, files in os.walk('cynic'):
    for file in files:
        if file.endswith('.py'):
            if clean_file(os.path.join(root, file)):
                count += 1

print(f"Cleaned {count} files.")
