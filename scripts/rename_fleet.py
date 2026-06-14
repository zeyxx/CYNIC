import os
import re

replacements = {
    r"\bcynic-forge\b": "cynic-host-proxmox",
    r"\bcynic-core\b": "cynic-host-proxmox",
    r"\bcynic-gpu\b": "cynic-host-gpu",
    r"\bcynic-node-forge\b": "cynic-vm-anvil",
    r"\bforgejo\b": "cynic-lxc-git",
    r"\bheadscale\b": "cynic-lxc-vpn"
}

def process_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except UnicodeDecodeError:
        return False

    new_content = content
    for pattern, replacement in replacements.items():
        new_content = re.sub(pattern, replacement, new_content)

    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated {filepath}")
        return True
    return False

for root, dirs, files in os.walk('.'):
    if '.git' in root or 'target' in root or '.next' in root or 'venv' in root or '__pycache__' in root:
        continue
    for file in files:
        if file.endswith(('.rs', '.md', '.toml', '.json', '.sh', '.py')):
            process_file(os.path.join(root, file))

