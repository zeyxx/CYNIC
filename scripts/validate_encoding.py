"""
Validate all Python files are valid UTF-8 with correct phi, times, sqrt, arrow symbols.

Exit code 1 if any file has encoding issues.
"""
import sys
from pathlib import Path


def validate_file(path: Path) -> list[str]:
    """Check file for UTF-8 validity and mathematical symbol correctness."""
    errors = []
    try:
        content = path.read_text(encoding='utf-8')
    except UnicodeDecodeError as e:
        errors.append(f"{path}: UTF-8 decode error at position {e.start}: {e.reason}")
        return errors

    # Check for common corruption patterns using hex codes
    # \xc3\x97 = corrupted times (×)
    # \xe2\x80\x9a = corrupted sqrt-like
    # \xe2\x86\x92 = corrupted arrow-like
    # \xce\xc6 = corrupted phi-like
    corruption_patterns = [
        '\xc3\x97',  # Ã—
        '\xe2\x80\x9a',  # â€š
        '\xe2\x86\x92',  # â†'
        '\xce\xc6',  # Ï†
    ]

    for pattern in corruption_patterns:
        if pattern in content:
            errors.append(f"{path}: Found Unicode corruption pattern")
            break

    return errors


def main():
    cynic_dir = Path("cynic")
    test_dir = Path("tests")
    all_errors = []

    for py_file in list(cynic_dir.rglob("*.py")) + list(test_dir.rglob("*.py")):
        all_errors.extend(validate_file(py_file))

    if all_errors:
        print("[FAIL] Encoding validation failed:")
        for error in all_errors:
            print(f"  {error}")
        sys.exit(1)
    else:
        print("[PASS] All files have valid UTF-8 encoding")
        sys.exit(0)


if __name__ == "__main__":
    main()
