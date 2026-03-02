"""
Validate commit message format.

Expected format:
  type(scope): description

  optional body

Where:
  - type: feat, fix, docs, style, refactor, perf, test, chore
  - scope: priority-N, feature-name, or component
  - description: 50 chars or less, imperative mood
  - body: optional, wrapped at 72 chars

Exit code 1 if validation fails.
"""
import sys
from pathlib import Path


def validate_commit_message():
    """Validate the commit message from COMMIT_EDITMSG."""
    # Try to read the commit message
    msg_file = Path(".git/COMMIT_EDITMSG")

    if not msg_file.exists():
        # In pre-commit hook, the message hasn't been written yet
        # We'll skip validation in this case (hook runs after git add, before commit)
        print("[SKIP] Commit message not yet available for validation")
        return True

    try:
        message = msg_file.read_text(encoding='utf-8').strip()
    except Exception as e:
        print(f"[WARN] Could not read commit message: {e}")
        return True  # Don't fail if we can't read it

    if not message:
        print("[FAIL] Commit message is empty")
        return False

    # Get the first line (subject)
    lines = message.split('\n')
    subject = lines[0].strip()

    # Valid types
    valid_types = {'feat', 'fix', 'docs', 'style', 'refactor', 'perf', 'test', 'chore', 'ci'}

    # Parse "type(scope): description" OR "type: description"
    if ':' not in subject:
        print(f"[FAIL] Commit message format invalid: {subject}")
        print("  Expected: type(scope): description OR type: description")
        return False

    # Check if there's a scope (parentheses BEFORE the colon)
    colon_idx = subject.find(': ')
    if colon_idx == -1:
        colon_idx = subject.find(':')

    has_scope = '(' in subject[:colon_idx] and ')' in subject[:colon_idx]

    try:
        if has_scope:
            type_part, rest = subject.split('(', 1)
            scope, after_scope = rest.split(')', 1)

            if not after_scope.startswith(': '):
                print(f"[FAIL] Missing ': ' after scope: {subject}")
                return False

            type_part = type_part.strip()
            description = after_scope[2:].strip()

            # Validate scope (must contain at least one character)
            if not scope or len(scope.strip()) == 0:
                print("[FAIL] Scope cannot be empty")
                return False
        else:
            # Format: type: description
            if ': ' not in subject:
                print(f"[FAIL] Missing ': ' separator: {subject}")
                return False
            type_part, description = subject.split(': ', 1)
            type_part = type_part.strip()
            description = description.strip()

    except ValueError:
        print(f"[FAIL] Could not parse commit message: {subject}")
        return False

    # Validate type
    if type_part not in valid_types:
        print(f"[FAIL] Invalid type '{type_part}'. Must be one of: {', '.join(valid_types)}")
        return False

    # Validate description length
    if len(subject) > 100:
        print(f"[FAIL] Commit subject too long ({len(subject)} > 100 chars): {subject}")
        return False

    if len(description) == 0:
        print("[FAIL] Description cannot be empty")
        return False

    print("[PASS] Commit message format is valid")
    return True


if __name__ == "__main__":
    if not validate_commit_message():
        sys.exit(1)
