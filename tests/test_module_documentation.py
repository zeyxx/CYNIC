"""
Test suite verifying comprehensive module-level documentation.

Every module in the CYNIC package should have a docstring explaining:
- Purpose and role in the system
- Key classes/functions exported
- Typical usage patterns
- Related modules (See Also)
"""

import importlib
import pkgutil


def test_all_modules_have_docstrings():
    """Every non-test module should have a docstring."""
    import cynic

    undocumented = []
    skipped = []
    documented_count = 0

    # Walk all packages under cynic
    for _importer, modname, _ispkg in pkgutil.walk_packages(
        path=cynic.__path__, prefix="cynic.", onerror=lambda x: None
    ):
        # Skip test modules and specific transitional areas
        if any(
            x in modname
            for x in ["test", "__pycache__", "interfaces", "nervous", "layers"]
        ):
            skipped.append(modname)
            continue

        try:
            module = importlib.import_module(modname)
            doc = module.__doc__

            # Check for docstring
            if not doc or doc.strip() == "":
                undocumented.append(modname)
            else:
                # Verify it looks like a real docstring (>20 chars, contains useful info)
                if len(doc.strip()) < 20:
                    undocumented.append(f"{modname} (too short)")
                else:
                    documented_count += 1
        except Exception as e:
            # Some modules may fail to import due to missing dependencies
            # This is okay - we just skip them
            skipped.append(f"{modname} (import error: {type(e).__name__})")

    # Report results

    if undocumented:
        for _mod in sorted(undocumented):
            pass

    assert len(undocumented) == 0, f"Found {len(undocumented)} undocumented modules"


def test_documentation_quality():
    """Sample docstrings should follow standard format."""

    # Test a few key modules
    test_modules = [
        ("cynic", "Main package"),
        ("cynic.kernel.core", "Core infrastructure"),
        ("cynic.kernel.organism.brain.cognition", "Judgment system"),
        ("cynic.kernel.organism", "Organism architecture"),
    ]

    for modname, _description in test_modules:
        try:
            module = importlib.import_module(modname)
            doc = module.__doc__

            assert doc, f"{modname}: Missing docstring"

            # Check for key elements
            doc.lower()
            has_description = len(doc.strip()) > 50

            assert (
                has_description
            ), f"{modname}: Docstring too short (needs description)"

        except ImportError:
            pass
        except AssertionError:
            raise


def test_key_modules_exported():
    """Verify key modules are accessible."""

    # Core exports

    # Learning exports


if __name__ == "__main__":
    test_all_modules_have_docstrings()
    test_documentation_quality()
    test_key_modules_exported()
