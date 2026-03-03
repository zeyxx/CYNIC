import os
import re

def repair_except_indentation(directory):
    # Pattern to find 'except' followed by a logger call on the next line without indentation
    # Using \r?\n to handle Windows/Unix line endings
    pattern = re.compile(r'(except\s+Exception\s+as\s+(_e|e):\s*)\r?\n\s*(logger\.(debug|error|warning)\()')
    
    count = 0
    for root, dirs, files in os.walk(directory):
        if '.git' in root or '__pycache__' in root:
            continue
            
        for file in files:
            if file.endswith('.py'):
                path = os.path.join(root, file)
                try:
                    with open(path, 'r', encoding='utf-8') as f:
                        content = f.read()
                    
                    if pattern.search(content):
                        # Add exactly 12 spaces of indentation to the logger call
                        new_content = pattern.sub(r'\1\n            \3', content)
                        with open(path, 'w', encoding='utf-8') as f:
                            f.write(new_content)
                        print(f"Fixed: {path}")
                        count += 1
                except Exception as e:
                    print(f"Error in {path}: {e}")
    
    print(f"Total files fixed: {count}")

if __name__ == "__main__":
    repair_except_indentation('.')
