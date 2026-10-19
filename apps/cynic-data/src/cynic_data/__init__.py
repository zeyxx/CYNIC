"""CYNIC data-centric framework.

Four layers, named by contract (not by metaphor):

    Capture    -> authoritative: exact bytes emitted by a harness, immutable, content-addressed
    Canonical  -> authoritative: typed harness-agnostic facts (turns, tools, reasoning, edges, state)
    Model      -> derived: the trajectory hypergraph, materialized and queryable
    Projection -> derived: consumer views (SFT, DPO, PRM, diffs), each with a declared loss

Law: a layer reads only from the layer above it. Capture and Canonical are never
overwritten. Model and Projection are disposable and reconstructible.
"""

__version__ = "0.1.0"
