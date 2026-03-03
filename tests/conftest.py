import sys
import os

# Ensure the project root is in PYTHONPATH
# We need to point to the PARENT of the 'tests' directory
root_path = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, root_path)

print(f"🌀 CYNIC Environment: Root set to {root_path}")
