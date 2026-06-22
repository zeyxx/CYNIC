# Quantum ECC Optimization Levers

Ordered by estimated impact on gate×qubit product. All estimates are directional.

## Lever 1: windowed_scalar_mult
**Impact:** 4–8× reduction (highest priority)
**What:** Replace scalar-bit-by-bit point addition (256 controlled ECPointAdds) with
w=4 windowed arithmetic. Precompute 16 = 2^4 multiples of the base point as classical
constants. Implement a QROM lookup: 4 address qubits load one of 16 (x,y) pairs into
ancilla registers. Loop: 64 iterations of (QROM lookup → unconditional ECPointAdd →
QROM uncompute). Net result: 256 controlled adds → 64 unconditional adds + 64 QROM lookups.
**Applies when:** `windowed == false` in InspectionResult.
**Constraint:** Every QROM lookup must uncompute cleanly (ancilla returns to |0⟩).
**Do NOT change:** field arithmetic, modular inversion, coordinate system.

## Lever 2: projective_coordinates
**Impact:** 2–4× reduction
**What:** Switch from affine to Jacobian projective coordinates. Affine ECPointAdd
requires modular inversion (λ = (y2−y1)/(x2−x1)) — extremely expensive (~30× a
multiplication). Jacobian coords use Z field to defer division; only one inversion at
the very end (convert back to affine for output). Jacobian formulas: ~10–12 field
multiplications per add, vs 7 affine + 1 inversion. Net win: eliminate 255 inversions.
Use the Brier-Joye unified addition formula for simplicity.
**Applies when:** `projective == false` in InspectionResult.
**Constraint:** Output must still return affine (x,y) point for correctness check.

## Lever 3: toffoli_ladder_ancilla_reuse
**Impact:** 1.3–2× reduction on qubit component
**What:** Replace fan-out Toffoli trees (one ancilla allocated per Toffoli) with
linear Toffoli ladders that reuse a single ancilla bit via compute-uncompute pattern.
Pattern: instead of `ccx(a, b, anc0); ccx(c, d, anc1); ccx(anc0, anc1, out)`,
use `ccx(a, b, anc); ccx(c, d, out); cx(anc, out); ccx(c, d, out); ccx(a, b, anc)`.
Scan for sequential CCX blocks in field multiplication subcircuits.
**Applies when:** always (local transformation, does not depend on global structure).
**Constraint:** Gate count increases slightly; net product must decrease.

## Lever 4: karatsuba_field_mult
**Impact:** 1.2–1.5× reduction on gate component
**What:** Replace schoolbook O(n²) modular multiplication with Karatsuba O(n^1.58).
For 256-bit fields, Karatsuba splits each operand into two 128-bit halves:
`a = a_hi * 2^128 + a_lo`, compute three 128-bit products, reconstruct.
Gate count: schoolbook ~n² = 65536 Toffoli; Karatsuba ~3*(n/2)^2 = ~49152 (~25% less).
Ancilla cost increases (need temp registers for intermediate products).
**Applies when:** field multiplication uses schoolbook method (inspect for nested loops
of depth ~n over n-bit operands).
**Constraint:** Must preserve modular reduction (mod secp256k1 prime p).
