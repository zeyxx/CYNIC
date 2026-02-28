"""
CYNIC Immune System Layer (Layer 9) — Defense and anomaly detection.

Part of the 10-layer organism architecture, the immune layer protects CYNIC
from malicious inputs, corrupted data, and hostile attack vectors.

Immune Functions:
    Input Validation: Detects malformed or weaponized cells
    Anomaly Detection: Identifies out-of-distribution behavior
    Rate Limiting: Defends against DoS attacks
    Signature Validation: Verifies judgment authenticity
    Reputation Veto: Blocks decisions from low E-Score sources

Threat Model:
    L0 (Byte): Malformed protocols
    L1 (Logic): Contradictory axiom violations
    L2 (Intent): Social engineering and manipulation
    L3 (Emergence): Coordinated attacks on consensus

Typical usage:
    from cynic.kernel.organism.layers import ImmuneSystem
    immune = ImmuneSystem()
    is_safe = immune.validate(cell)

See Also:
    cynic.kernel.organism.layers: 10-layer organism architecture
    cynic.kernel.core.axioms: Axiom violations trigger immune response
    cynic.brain.cognition.cortex: Immune veto on judgment output
"""
