import re
from pathlib import Path


def migrate_config_imports():
    patterns = [
        (
            r"from cynic\.kernel\.core\.config import CynicConfig",
            "from cynic.config import CynicConfig",
        ),
        (
            r"from cynic\.kernel\.core\.config import get_config",
            "from cynic.config import get_config",
        ),
        (r"import cynic\.kernel\.core\.config", "import cynic.config"),
        (
            r"from cynic\.kernel\.core\.config import CynicConfig as _CynicConfig",
            "from cynic.config import CynicConfig as _CynicConfig",
        ),
    ]

    count = 0
    # Scan all py files in the root and cynic/
    files = list(Path(".").rglob("*.py"))

    for file_path in files:
        if "__pycache__" in str(file_path):
            continue

        try:
            content = file_path.read_text(encoding="utf-8", errors="ignore")
            new_content = content

            for pattern, replacement in patterns:
                new_content = re.sub(pattern, replacement, new_content)

            if new_content != content:
                file_path.write_text(new_content, encoding="utf-8")
                print(f"Migrated: {file_path}")
                count += 1
        except Exception as e:
            print(f"Error in {file_path}: {e}")

    print(f"\nMigration complete. {count} files updated.")


if __name__ == "__main__":
    migrate_config_imports()
