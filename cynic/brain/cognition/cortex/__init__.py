"""
CYNIC Cortex — Fractal judgment orchestration and decision pipeline.

The cortex orchestrates judgment flows by:
1. Selecting consciousness level (REFLEX/MICRO/MACRO) based on cell complexity
2. Routing to appropriate cycle handler (cycle_reflex, cycle_micro, cycle_macro)
3. Composing handler chains via HandlerComposer
4. Executing action based on final judgment

Key Components:
    orchestrator: JudgmentPipeline and JudgmentOrchestrator
    handlers: BaseHandler, HandlerRegistry, HandlerComposer
    composer: Chains handlers in optimal order

Cycle Levels:
    REFLEX: <10ms non-LLM decision (axiom checks, rule-based)
    MICRO: ~500ms fast LLM judgment (single Dog + axioms)
    MACRO: ~2.85s full pipeline (11 Dogs vote + consensus)

Typical usage:
    from cynic.brain.cognition.cortex import JudgmentOrchestrator
    orchestrator = JudgmentOrchestrator(registry)
    verdict = await orchestrator.judge(cell)

See Also:
    cynic.brain.cognition.neurons: 11 Dog judges
    cynic.interfaces.api.handlers.judgment_executor: Executes judgment decisions
"""
