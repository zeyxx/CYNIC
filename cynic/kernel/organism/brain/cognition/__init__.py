"""
CYNIC Cognition — Judgment orchestration and decision-making cortex.

Houses the fractally-organized judgment pipeline that processes cells through:
1. Level selection (which consciousness level is appropriate?)
2. Cycle execution (REFLEX/MICRO/MACRO judgment paths)
3. Dog voting (11 Sefirot judges vote on decision)
4. Verdict composition (HOWL/WAG/GROWL/BARK Q-Score)

Architecture:
    cortex: Judgment pipeline orchestrator and handler registry
    neurons: 11 Dogs (judge implementations) — SEFIROT-based
    handlers: Individual judgment decision handlers

Typical usage:
    from cynic.kernel.organism.brain.cognition import JudgmentPipeline, JudgmentOrchestrator
    pipeline = JudgmentPipeline(cell=governance_cell)
    judgment = await orchestrator.judge(pipeline)

See Also:
    cynic.kernel.core.consciousness: Consciousness level definitions
    cynic.kernel.organism: Organism architecture using cognition layer
    cynic.judges: Judge implementations and contracts
"""
