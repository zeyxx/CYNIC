"""CYNIC Assembly — Compose primitives into workflows."""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable, Optional
from functools import wraps
import asyncio

from .primitives import (
    DNA_Cell,
    DNA_Judgment,
    DNA_Decision,
    DNA_Result,
    PERCEIVE,
    JUDGE,
    DECIDE,
    ACT,
    LEARN,
)


# ============================================================================
# WORKFLOW CLASS (Compose primitives)
# ============================================================================

@dataclass
class Workflow:
    """
    Compose CYNIC primitives into a workflow.
    Think of it as a pipeline: PERCEIVE → JUDGE → DECIDE → ACT → LEARN.
    """

    name: str
    steps: list = None  # List of (primitive, kwargs) tuples

    def __post_init__(self):
        if self.steps is None:
            self.steps = []

    def perceive(self, source: str, **kwargs) -> Workflow:
        """Add PERCEIVE step."""
        self.steps.append(("perceive", {"source": source, **kwargs}))
        return self

    def judge(self, level: str = "MACRO", **kwargs) -> Workflow:
        """Add JUDGE step."""
        self.steps.append(("judge", {"level": level, **kwargs}))
        return self

    def decide(self, axiom: str = "PHI", **kwargs) -> Workflow:
        """Add DECIDE step."""
        self.steps.append(("decide", {"axiom": axiom, **kwargs}))
        return self

    def act(self, executor: str = "report", **kwargs) -> Workflow:
        """Add ACT step."""
        self.steps.append(("act", {"executor": executor, **kwargs}))
        return self

    def learn(self, signal: str = "success", **kwargs) -> Workflow:
        """Add LEARN step."""
        self.steps.append(("learn", {"signal": signal, **kwargs}))
        return self

    async def run(
        self,
        input_data: str | dict,
        orchestrator: Any = None,
        qtable: Any = None,
    ) -> dict:
        """
        Execute the workflow.

        Args:
            input_data: Initial data (string content or dict with content + metadata)
            orchestrator: CYNIC orchestrator (for JUDGE step)
            qtable: CYNIC QTable (for LEARN step)

        Returns:
            Workflow result dict
        """
        # Parse input
        if isinstance(input_data, str):
            content = input_data
            metadata = {}
        elif isinstance(input_data, dict):
            content = input_data.get("content", "")
            metadata = input_data.get("metadata", {})
        else:
            raise TypeError(f"input_data must be str or dict, got {type(input_data)}")

        # Execute steps
        cell = None
        judgment = None
        decision = None
        act_result = None
        learn_result = None

        for step_name, step_kwargs in self.steps:
            if step_name == "perceive":
                source = step_kwargs.pop("source")
                cell = await PERCEIVE(source, content, metadata=metadata)

            elif step_name == "judge":
                if cell is None:
                    raise RuntimeError("JUDGE requires PERCEIVE step first")
                level = step_kwargs.pop("level", "MACRO")
                judgment = await JUDGE(cell, level=level, orchestrator=orchestrator)

            elif step_name == "decide":
                if judgment is None:
                    raise RuntimeError("DECIDE requires JUDGE step first")
                axiom = step_kwargs.pop("axiom", "PHI")
                decision = DECIDE(judgment, axiom=axiom)

            elif step_name == "act":
                if decision is None:
                    raise RuntimeError("ACT requires DECIDE step first")
                executor = step_kwargs.pop("executor", "report")
                act_result = await ACT(decision, executor=executor)

            elif step_name == "learn":
                if act_result is None:
                    raise RuntimeError("LEARN requires ACT step first")
                signal = step_kwargs.pop("signal", "success")
                learn_result = await LEARN(act_result, signal=signal, qtable=qtable)

        return {
            "workflow_name": self.name,
            "cell": cell,
            "judgment": judgment,
            "decision": decision,
            "act_result": act_result,
            "learn_result": learn_result,
        }


# ============================================================================
# DECORATOR FOR WORKFLOWS (Simple syntax)
# ============================================================================

def cynic_workflow(name: str = None):
    """
    Decorator to create a workflow from a function.

    Example:
        @cynic_workflow("ANALYZE_CODE")
        async def my_workflow(content: str, orchestrator=None, qtable=None):
            cell = await PERCEIVE("code", content)
            judgment = await JUDGE(cell, orchestrator=orchestrator)
            decision = DECIDE(judgment, axiom="VERIFY")
            result = await ACT(decision, executor="report")
            await LEARN(result, qtable=qtable)
            return result
    """

    def decorator(func: Callable):
        @wraps(func)
        async def wrapper(*args, **kwargs):
            return await func(*args, **kwargs)

        wrapper._is_cynic_workflow = True
        wrapper._workflow_name = name or func.__name__
        return wrapper

    return decorator


# ============================================================================
# BUILT-IN WORKFLOWS (Common patterns)
# ============================================================================

async def AUDIT_REPO(
    repo_path: str,
    orchestrator: Any = None,
    qtable: Any = None,
) -> dict:
    """
    Built-in workflow: AUDIT_REPO
    Analyze repository for health/quality/patterns.

    Uses primitives: PERCEIVE(git) → JUDGE(MACRO) → DECIDE(CULTURE) → ACT(report)
    """
    workflow = (
        Workflow("AUDIT_REPO")
        .perceive(source="git", context=f"Repository: {repo_path}")
        .judge(level="MACRO")
        .decide(axiom="CULTURE")
        .act(executor="report")
        .learn(signal="success")
    )
    return await workflow.run(repo_path, orchestrator=orchestrator, qtable=qtable)


async def ANALYZE_CODE_SECURITY(
    code: str,
    orchestrator: Any = None,
    qtable: Any = None,
) -> dict:
    """
    Built-in workflow: ANALYZE_CODE_SECURITY
    Analyze code for security vulnerabilities.

    Uses primitives: PERCEIVE(code) → JUDGE(MACRO) → DECIDE(VERIFY) → ACT(alert)
    """
    workflow = (
        Workflow("ANALYZE_CODE_SECURITY")
        .perceive(source="code", context="Security analysis")
        .judge(level="MACRO")
        .decide(axiom="VERIFY")
        .act(executor="alert")
        .learn(signal="success")
    )
    return await workflow.run(code, orchestrator=orchestrator, qtable=qtable)


async def FAST_QUALITY_CHECK(
    code: str,
    orchestrator: Any = None,
    qtable: Any = None,
) -> dict:
    """
    Built-in workflow: FAST_QUALITY_CHECK
    Quick code quality check (low latency).

    Uses primitives: PERCEIVE(code) → JUDGE(REFLEX) → DECIDE(BURN) → ACT(report)
    """
    workflow = (
        Workflow("FAST_QUALITY_CHECK")
        .perceive(source="code", context="Quick quality check")
        .judge(level="REFLEX")  # Fast, shallow
        .decide(axiom="BURN")  # Simplicity matters
        .act(executor="report")
        .learn(signal="success")
    )
    return await workflow.run(code, orchestrator=orchestrator, qtable=qtable)


async def CONTINUOUS_LEARNING(
    feedback: str,
    orchestrator: Any = None,
    qtable: Any = None,
) -> dict:
    """
    Built-in workflow: CONTINUOUS_LEARNING
    Process human feedback and update learning.

    Uses primitives: PERCEIVE(social) → JUDGE(MICRO) → DECIDE(FIDELITY) → ACT(learn)
    """
    workflow = (
        Workflow("CONTINUOUS_LEARNING")
        .perceive(source="social", context="Human feedback")
        .judge(level="MICRO")  # Medium depth
        .decide(axiom="FIDELITY")  # Truth-seeking
        .act(executor="learn")
        .learn(signal="human_feedback")
    )
    return await workflow.run(feedback, orchestrator=orchestrator, qtable=qtable)


# ============================================================================
# UTILITY FUNCTIONS
# ============================================================================

def list_workflows() -> list[str]:
    """List all available workflows."""
    return [
        "AUDIT_REPO",
        "ANALYZE_CODE_SECURITY",
        "FAST_QUALITY_CHECK",
        "CONTINUOUS_LEARNING",
    ]


def create_custom_workflow(
    name: str,
    steps: list[tuple[str, dict]],
) -> Workflow:
    """
    Create a custom workflow from steps.

    Example:
        workflow = create_custom_workflow(
            "MY_WORKFLOW",
            [
                ("perceive", {"source": "code"}),
                ("judge", {"level": "MACRO"}),
                ("decide", {"axiom": "VERIFY"}),
                ("act", {"executor": "report"}),
            ]
        )
        result = await workflow.run(content, orchestrator=orch, qtable=qtable)
    """
    workflow = Workflow(name)
    for step_name, step_kwargs in steps:
        if step_name == "perceive":
            workflow.perceive(**step_kwargs)
        elif step_name == "judge":
            workflow.judge(**step_kwargs)
        elif step_name == "decide":
            workflow.decide(**step_kwargs)
        elif step_name == "act":
            workflow.act(**step_kwargs)
        elif step_name == "learn":
            workflow.learn(**step_kwargs)
    return workflow
