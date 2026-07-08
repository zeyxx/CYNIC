# CYNIC Multi-Scale Empirical Rule

Status: normative foundation.

Machine contract: `axioms/multiscale-organism-property-map-v1.json`.

This document explains how to read the empirical multi-scale property map. The
goal is not to invent a new metaphor for the organism. The goal is to keep the
same property nucleus observable across scales without turning a physical lower
bound into a governance layer.

## 1. Rule

The same property may appear at many scales.

What changes with scale is:

- the representation of the property;
- the evidence used to verify it;
- the owner allowed to mutate it;
- the level at which it stops being authority.

This means:

- organism-level identity is governance and lifecycle;
- organ-level identity is bounded function;
- cell-level identity is execution;
- physical-system identity is material constraint;
- Planck-limit language is only a description limit.

## 2. Empirical Mapping

The machine contract groups each scale with a property nucleus:

- identity
- boundary
- state
- energy
- flux
- memory
- observation
- action
- dependency
- invariant
- failure_mode

The scale order is a projection chain, not just a list. Each scale except
`organism` names the more abstract parent scale it refines.

Each property should be checked against evidence, not intuition.
Examples of evidence are:

- source-of-truth docs;
- registry state;
- runtime outputs;
- validators;
- health checks;
- observed logs.

## 3. What "Planck" Means Here

The Planck limit is not a governance surface.

It is the point where the organism analogy stops being useful as an operational
description. If a statement about the organism requires Planck-scale language to
make a governance claim, the statement is overreaching.

## 4. How To Use It

Use the property map when you need to answer:

- what remains invariant across scales;
- what must be re-represented;
- which file or runtime owns the evidence;
- whether a proposed abstraction is empirical or merely poetic.

Do not use it to justify hidden coupling. The purpose is to expose it.
