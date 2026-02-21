"""CYNIC DNA Examples — Real usage patterns."""
from __future__ import annotations

import asyncio
from .primitives import PERCEIVE, JUDGE, DECIDE, ACT, LEARN, DNA_Cell
from .assembly import (
    Workflow,
    AUDIT_REPO,
    ANALYZE_CODE_SECURITY,
    FAST_QUALITY_CHECK,
    CONTINUOUS_LEARNING,
    cynic_workflow,
)


# ============================================================================
# EXAMPLE 1: Simple Linear Chain
# ============================================================================

async def example_linear_chain():
    """
    Simplest possible usage: perceive → judge → decide → act

    This is what you'd do if you have orchestrator + qtable available.
    """
    print("\n=== EXAMPLE 1: Linear Chain ===")

    # Assume orchestrator and qtable are available (from app state)
    orchestrator = None  # Would be injected from FastAPI state
    qtable = None

    # STEP 1: Perceive
    cell = await PERCEIVE(
        source="code",
        content="def hello(): print('world')",
        metadata={"file": "hello.py"},
    )
    print(f"✓ PERCEIVE: {cell.id} ({cell.source})")

    # STEP 2: Judge (if orchestrator available)
    if orchestrator:
        judgment = await JUDGE(cell, level="REFLEX", orchestrator=orchestrator)
        print(f"✓ JUDGE: Q={judgment.q_score:.1f} Verdict={judgment.verdict}")

        # STEP 3: Decide
        decision = DECIDE(judgment, axiom="VERIFY")
        print(f"✓ DECIDE: Action={decision.action_type}")

        # STEP 4: Act
        result = await ACT(decision, executor="report")
        print(f"✓ ACT: {result.status} - {result.output}")

        # STEP 5: Learn
        learn_result = await LEARN(result, signal="success", qtable=qtable)
        print(f"✓ LEARN: {learn_result}")
    else:
        print("(Skipping JUDGE-LEARN without orchestrator)")


# ============================================================================
# EXAMPLE 2: Using Built-in Workflows
# ============================================================================

async def example_builtin_workflows():
    """
    Use pre-built workflows for common tasks.
    No need to compose primitives manually.
    """
    print("\n=== EXAMPLE 2: Built-in Workflows ===")

    orchestrator = None  # Would be injected
    qtable = None

    # Workflow 1: Quick code quality check
    print("\n1. FAST_QUALITY_CHECK workflow:")
    result = await FAST_QUALITY_CHECK(
        code="def bad( x,y ):  return x+y",
        orchestrator=orchestrator,
        qtable=qtable,
    )
    print(f"   Cell: {result['cell'].id if result.get('cell') else 'N/A'}")
    if result.get("judgment"):
        print(f"   Q-Score: {result['judgment'].q_score}")

    # Workflow 2: Security analysis
    print("\n2. ANALYZE_CODE_SECURITY workflow:")
    result = await ANALYZE_CODE_SECURITY(
        code="def secure_function(): pass  # Placeholder",
        orchestrator=orchestrator,
        qtable=qtable,
    )
    print(f"   Action: {result.get('decision').action_type if result.get('decision') else 'N/A'}")

    # Workflow 3: Repository audit
    print("\n3. AUDIT_REPO workflow:")
    result = await AUDIT_REPO(
        repo_path="/path/to/repo",
        orchestrator=orchestrator,
        qtable=qtable,
    )
    print(f"   Workflow: {result['workflow_name']}")


# ============================================================================
# EXAMPLE 3: Custom Workflow (DIY)
# ============================================================================

async def example_custom_workflow():
    """
    Build your own workflow by chaining primitives.
    """
    print("\n=== EXAMPLE 3: Custom Workflow ===")

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

    print(f"Created workflow: {workflow.name}")
    print(f"Steps: {[s[0] for s in workflow.steps]}")

    # Execute it
    code = """
    // Hypothetical contract structure
    pub fn transfer(amount: u64) {
        // Implementation here
    }
    """

    result = await workflow.run(code, orchestrator=orchestrator, qtable=qtable)
    print(f"Result: {result['workflow_name']} completed")


# ============================================================================
# EXAMPLE 4: API Integration Pattern
# ============================================================================

async def example_api_integration():
    """
    How to use DNA primitives in a FastAPI handler.
    This is what you'd see in a real endpoint.
    """
    print("\n=== EXAMPLE 4: API Integration ===")

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
    response = await handle_analyze_code("def hello(): pass")
    print(f"API Response: {response}")


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
    print(f"Perceived: {cell.id}")

    if orchestrator:
        judgment = await JUDGE(cell, level="MACRO", orchestrator=orchestrator)
        print(f"Judged: Q={judgment.q_score}")

        decision = DECIDE(judgment, axiom="CULTURE")
        print(f"Decided: {decision.action_type}")

        result = await ACT(decision, executor="report")
        print(f"Acted: {result.status}")

        await LEARN(result, signal="success", qtable=qtable)
        print(f"Learned")

        return result
    return None


async def example_decorator_usage():
    """Use the @cynic_workflow decorated function."""
    print("\n=== EXAMPLE 5: Decorator Pattern ===")

    orchestrator = None
    qtable = None

    result = await my_custom_audit(
        content="def audit_me(): pass",
        orchestrator=orchestrator,
        qtable=qtable,
    )
    print(f"Decorator workflow result: {result}")


# ============================================================================
# RUN ALL EXAMPLES
# ============================================================================

async def main():
    """Run all examples."""
    print("=" * 70)
    print("CYNIC DNA PRIMITIVES - EXAMPLES")
    print("=" * 70)

    await example_linear_chain()
    await example_builtin_workflows()
    await example_custom_workflow()
    await example_api_integration()
    await example_decorator_usage()

    print("\n" + "=" * 70)
    print("EXAMPLES COMPLETE")
    print("=" * 70)


if __name__ == "__main__":
    asyncio.run(main())
