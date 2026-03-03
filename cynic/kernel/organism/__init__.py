"""
CYNIC Organism - Integrated Biological Systems.

Layout:
-- handlers/             Event handlers (Judgment, Senses, etc.)
-- state_manager.py      Unified state (RAM/DB/File)
-- organism.py           Central coordinator and factory (awaken)
"""

from .organism import awaken, create_organism, Organism

__all__ = ["awaken", "create_organism", "Organism"]
