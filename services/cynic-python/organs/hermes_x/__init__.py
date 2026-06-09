"""
Hermes X Organ — Data-driven perception, analysis, and learning.

A proper organ module following the 7-layer data architecture:
1. PERCEPTION (sensors.py)
2. TRANSFORMATION (transformation.py)
3. STRUCTURATION (schema.py)
4. ANALYSE & COMPRÉHENSION (analysis.py)
5. APPRENTISSAGE (learning.py)
6. FIABILITÉ (error handling, atomicity)
7. GOUVERNANCE (audit, lineage)

Owns: ~/.cynic/organs/hermes/x/
Entry point: python -m cynic_python.organs.hermes_x
"""

from .run_cycle import HermesXOrgan, main

__all__ = ["HermesXOrgan", "main"]
