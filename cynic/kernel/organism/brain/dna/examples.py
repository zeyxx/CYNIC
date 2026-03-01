"""CYNIC DNA Examples â€” Real usage patterns."""

from __future__ import annotations

import asyncio

from .assembly import (
    ANALYZE_CODE_SECURITY,
    AUDIT_REPO,
    FAST_QUALITY_CHECK,
    Workflow,
    cynic_workflow,
)
from .primitives import ACT, DECIDE, JUDGE, LEARN, PERCEIVE

# ============================================================================
# EXAMPLE 1: Simple Linear Chain
# ============================================================================


async def example_linear_chain():
    """
    Simplest possible usage: perceive â†’ judge â†’ decide â†’ act

    This is what you'd do if you have orchestrator + qtable available.
    """

    # Assume orchestrator and qtable are available (from app state)
    orchestrator = None  # Would be injected from FastAPI state
    qtable = None

    # STEP 1: Perceive
    cell = await PERCEIVE(
        source="code",
        content="def hello(): print('world')",
        metadata={"file": "hello.py"},
    )

    # STEP 2: Judge (if orchestrator available)
    if orchestrator:
        judgment = await JUDGE(cell, level="REFLEX", orchestrator=orchestrator)

        # STEP 3: Decide
        decision = DECIDE(judgment, axiom="VERIFY")

        # STEP 4: Act
        result = await ACT(decision, executor="report")

        # STEP 5: Learn
        await LEARN(result, signal="success", qtable=qtable)
    else:
        pass


# ============================================================================
# EXAMPLE 2: Using Built-in Workflows
# ============================================================================


async def example_builtin_workflows():
    """
    Use pre-built workflows for common tasks.
    No need to compose primitives manually.
    """

    orchestrator = None  # Would be injected
    qtable = None

    # Workflow 1: Quick code quality check
    result = await FAST_QUALITY_CHECK(
        code="def bad( x,y ):  return x+y",
        orchestrator=orchestrator,
        qtable=qtable,
    )
    if result.get("judgment"):
        pass

    # Workflow 2: Security analysis
    result = await ANALYZE_CODE_SECURITY(
        code="def secure_function(): pass  # Placeholder",
        orchestrator=orchestrator,
        qtable=qtable,
    )

    # Workflow 3: Repository audit
    result = await AUDIT_REPO(
        repo_path="/path/to/repo",
        orchestrator=orchestrator,
        qtable=qtable,
    )


# ============================================================================
# EXAMPLE 3: Custom Workflow (DIY)
# ============================================================================


async def example_custom_workflow():
    """
    Build your own workflow by chaining primitives.
    """

    orchestrator = None  # Would be injected
    qtable = None

    # Create custom workflow step-by-step
    workflow = (
        Workflow("CUSTOM_SOLANA_AUDIT")
        .perceive(source="code", context="Solana contract audit")
        .judge(level="MACRO")  # Deep analysis
        .decide(axiom="VERIFY")  # Focus on correctness
        .act(executor="report")  # Generate report
        .learn(signal="success")  # Learn from result
    )

    # Execute it
    code = """
    // Hypothetical contract structure
    pub fn transfer(amount: u64) {
        // Implementation here
    }
    """

    await workflow.run(code, orchestrator=orchestrator, qtable=qtable)


# ============================================================================
# EXAMPLE 4: API Integration Pattern
# ============================================================================


async def example_api_integration():
    """
    How to use DNA primitives in a FastAPI handler.
    This is what you'd see in a real endpoint.
    """

    # Simulating FastAPI dependency injection
    orchestrator = None  # Injected from app.state.orchestrator
    qtable = None  # Injected from app.state.qtable

    # This is what an endpoint would do:
    async def handle_analyze_code(code: str):
        """API endpoint: POST /analyze"""
        try:
            # Quick quality check
            result = await FAST_QUALITY_CHECK(
                code=code,
                orchestrator=orchestrator,
                qtable=qtable,
            )

            # Return the judgment to user
            if result.get("judgment"):
                return {
                    "status": "ok",
                    "q_score": result["judgment"].q_score,
                    "verdict": result["judgment"].verdict,
                    "reasoning": result["judgment"].reasoning,
                }
            else:
                return {"status": "error", "message": "Could not judge code"}

        except httpx.RequestError as e:
            return {"status": "error", "message": str(e)}

    # Simulate API call
    await handle_analyze_code("def hello(): pass")


# ============================================================================
# EXAMPLE 5: Decorator Pattern (Pythonic)
# ============================================================================


@cynic_workflow("MY_CUSTOM_AUDIT")
async def my_custom_audit(
    content: str,
    orchestrator=None,
    qtable=None,
):
    """
    Define a workflow using decorator (cleanest syntax).
    You can now call this like a normal async function.
    """
    cell = await PERCEIVE("code", content, metadata={"type": "audit"})

    if orchestrator:
        judgment = await JUDGE(cell, level="MACRO", orchestrator=orchestrator)

        decision = DECIDE(judgment, axiom="CULTURE")

        result = await ACT(decision, executor="report")

        await LEARN(result, signal="success", qtable=qtable)

        return result
    return None


async def example_decorator_usage():
    """Use the @cynic_workflow decorated function."""

    orchestrator = None
    qtable = None

    await my_custom_audit(
        content="def audit_me(): pass",
        orchestrator=orchestrator,
        qtable=qtable,
    )


# ============================================================================
# RUN ALL EXAMPLES
# ============================================================================


async def main():
    """Run all examples."""

    await example_linear_chain()
    await example_builtin_workflows()
    await example_custom_workflow()
    await example_api_integration()
    await example_decorator_usage()


if __name__ == "__main__":
    asyncio.run(main())
