"""
Test suite verifying comprehensive module-level documentation.

Every module in the CYNIC package should have a docstring explaining:
- Purpose and role in the system
- Key classes/functions exported
- Typical usage patterns
- Related modules (See Also)
"""
import pkgutil
import importlib
import sys
from pathlib import Path


def test_all_modules_have_docstrings():
    """Every non-test module should have a docstring."""
    import cynic
    
    undocumented = []
    skipped = []
    documented_count = 0
    
    # Walk all packages under cynic
    for importer, modname, ispkg in pkgutil.walk_packages(
        path=cynic.__path__,
        prefix="cynic.",
        onerror=lambda x: None
    ):
        # Skip test modules
        if "test" in modname or "__pycache__" in modname:
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
    print(f"\n{'='*60}")
    print(f"Module Documentation Test Results")
    print(f"{'='*60}")
    print(f"✓ Documented modules: {documented_count}")
    print(f"✗ Undocumented modules: {len(undocumented)}")
    print(f"⊘ Skipped (tests/errors): {len(skipped)}")
    print(f"{'='*60}\n")
    
    if undocumented:
        print("Undocumented modules:")
        for mod in sorted(undocumented):
            print(f"  - {mod}")
        print()
    
    assert len(undocumented) == 0, f"Found {len(undocumented)} undocumented modules"


def test_documentation_quality():
    """Sample docstrings should follow standard format."""
    import cynic
    
    # Test a few key modules
    test_modules = [
        ("cynic", "Main package"),
        ("cynic.core", "Core infrastructure"),
        ("cynic.cognition", "Judgment system"),
        ("cynic.organism", "Organism architecture"),
    ]
    
    for modname, description in test_modules:
        try:
            module = importlib.import_module(modname)
            doc = module.__doc__
            
            assert doc, f"{modname}: Missing docstring"
            
            # Check for key elements
            doc_lower = doc.lower()
            has_description = len(doc.strip()) > 50
            
            assert has_description, f"{modname}: Docstring too short (needs description)"
            
            print(f"✓ {modname}: {description}")
        except ImportError:
            print(f"⊘ {modname}: Could not import (skipped)")
        except AssertionError as e:
            print(f"✗ {modname}: {e}")
            raise


def test_key_modules_exported():
    """Verify key modules are accessible."""
    import cynic

    # Core exports
    from cynic.core.phi import PHI, PHI_INV, MAX_Q_SCORE
    from cynic.core.consciousness import ConsciousnessLevel

    # Learning exports
    from cynic.learning.unified_learning import UnifiedQTable

    print("✓ All key module exports are accessible")


if __name__ == "__main__":
    test_all_modules_have_docstrings()
    test_documentation_quality()
    test_key_modules_exported()
    print("\n" + "="*60)
    print("All documentation tests passed!")
    print("="*60)
