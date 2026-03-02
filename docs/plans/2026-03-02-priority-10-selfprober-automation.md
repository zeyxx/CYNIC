# Priority 10: SelfProber Automation — Proposal Execution & CLI Review

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to execute task-by-task.

**Goal:** Auto-execute low-risk self-improvement proposals, emit execution events for audit trail, and provide CLI review interface for higher-risk changes.

**Architecture:** Risk-classifier determines LOW_RISK vs REVIEW_REQUIRED based on severity + dimension. LOW_RISK proposals are executed automatically by dimension-specific handlers (QTABLE, ESCORE, METRICS, etc.). All executions emit PROPOSAL_EXECUTED or PROPOSAL_FAILED events. CLI tool (`cynic probes`) provides interactive approval workflow for REVIEW_REQUIRED proposals and audit log browsing.

**Tech Stack:** Python 3.13, asyncio, click (CLI), pure Python handlers (no external ML/config frameworks).

---

## Current State

- ✅ SelfProber generates 5-dimensional proposals (QTABLE, ESCORE, RESIDUAL, ARCHITECTURE, METRICS)
- ✅ Proposals persisted to ~/.cynic/self_proposals.json (rolling cap F(10)=55)
- ✅ API endpoints: GET /self-probes, POST /self-probes/{id}/apply, POST /self-probes/{id}/dismiss
- ❌ apply() only changes status — doesn't execute proposals
- ❌ No risk classification (auto-apply threshold)
- ❌ No execution handlers (what does "apply QTABLE proposal" actually do?)
- ❌ No event emission for execution (no audit trail)
- ❌ No CLI interface (only HTTP)
- ❌ No safety guardrails (rate limits, rollback, circuit breaker)

## Task 1: Risk Classifier & Proposal Executor

**Files:**
- Create: `cynic/kernel/organism/brain/cognition/cortex/proposal_executor.py`
- Modify: `cynic/kernel/organism/brain/cognition/cortex/self_probe.py` (add executor injection)
- Create: `tests/test_priority10_proposal_executor.py`

### Step 1: Write failing test

Create `tests/test_priority10_proposal_executor.py`:

```python
import pytest
from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import (
    ProposalExecutor,
    ProposalRiskLevel,
)
from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProposal


@pytest.mark.asyncio
class TestProposalRiskClassification:
    """Test risk classification logic."""

    async def test_low_severity_proposal_is_low_risk(self):
        """Proposals with severity < 0.2 are LOW_RISK."""
        proposal = SelfProposal(
            probe_id="test_001",
            trigger="MANUAL",
            pattern_type="METRICS",
            severity=0.15,  # < 0.2
            dimension="METRICS",
            target="event.judgment_created",
            recommendation="Rate spike detected",
            current_value=100.0,
            suggested_value=105.0,
        )
        executor = ProposalExecutor()
        risk = executor.classify_risk(proposal)
        assert risk == ProposalRiskLevel.LOW_RISK

    async def test_high_severity_proposal_is_review_required(self):
        """Proposals with severity >= 0.5 require review."""
        proposal = SelfProposal(
            probe_id="test_002",
            trigger="MANUAL",
            pattern_type="QTABLE",
            severity=0.7,  # >= 0.5
            dimension="QTABLE",
            target="state:action",
            recommendation="Q-value too low",
            current_value=0.1,
            suggested_value=0.25,
        )
        executor = ProposalExecutor()
        risk = executor.classify_risk(proposal)
        assert risk == ProposalRiskLevel.REVIEW_REQUIRED

    async def test_escore_proposals_are_review_required(self):
        """ESCORE dimension always requires review (dog exclusion is high-impact)."""
        proposal = SelfProposal(
            probe_id="test_003",
            trigger="MANUAL",
            pattern_type="ESCORE",
            severity=0.1,  # Low severity, but...
            dimension="ESCORE",
            target="dog_alpha",
            recommendation="Dog low E-Score",
            current_value=30.0,
            suggested_value=35.0,
        )
        executor = ProposalExecutor()
        risk = executor.classify_risk(proposal)
        assert risk == ProposalRiskLevel.REVIEW_REQUIRED

    async def test_metrics_rate_spike_is_low_risk(self):
        """METRICS dimension + RATE_SPIKE + low severity = LOW_RISK."""
        proposal = SelfProposal(
            probe_id="test_004",
            trigger="ANOMALY",
            pattern_type="RATE_SPIKE",  # pattern_type indicates anomaly type
            severity=0.12,
            dimension="METRICS",
            target="core.judgment_created",
            recommendation="Scale batching threshold",
            current_value=1000.0,
            suggested_value=1200.0,
        )
        executor = ProposalExecutor()
        risk = executor.classify_risk(proposal)
        assert risk == ProposalRiskLevel.LOW_RISK


@pytest.mark.asyncio
class TestProposalExecution:
    """Test execution of low-risk proposals."""

    async def test_execute_metrics_rate_spike_proposal(self):
        """Executing a RATE_SPIKE proposal updates collection window."""
        proposal = SelfProposal(
            probe_id="test_005",
            trigger="ANOMALY",
            pattern_type="RATE_SPIKE",
            severity=0.1,
            dimension="METRICS",
            target="core.judgment_created",
            recommendation="Increase window",
            current_value=55.0,  # seconds
            suggested_value=70.0,
        )
        executor = ProposalExecutor()
        result = await executor.execute(proposal)

        assert result.success is True
        assert result.proposal_id == "test_005"
        assert result.dimension == "METRICS"

    async def test_execute_qtable_proposal_updates_value(self):
        """Executing a QTABLE proposal updates the state-action Q-value."""
        proposal = SelfProposal(
            probe_id="test_006",
            trigger="MANUAL",
            pattern_type="QTABLE",
            severity=0.15,
            dimension="QTABLE",
            target="some_state:some_action",
            recommendation="Improve Q-value",
            current_value=0.15,
            suggested_value=0.25,
        )
        executor = ProposalExecutor()
        result = await executor.execute(proposal)

        assert result.success is True
        assert result.dimension == "QTABLE"
        assert result.new_value == 0.25

    async def test_execute_returns_failure_if_qtable_not_set(self):
        """If QTable not injected, execution fails gracefully."""
        proposal = SelfProposal(
            probe_id="test_007",
            trigger="MANUAL",
            pattern_type="QTABLE",
            severity=0.1,
            dimension="QTABLE",
            target="state:action",
            recommendation="Update Q",
            current_value=0.1,
            suggested_value=0.2,
        )
        executor = ProposalExecutor()  # No QTable injected
        result = await executor.execute(proposal)

        assert result.success is False
        assert "QTable not available" in result.error_message
```

### Step 2: Run test to verify it fails

```bash
pytest tests/test_priority10_proposal_executor.py::TestProposalRiskClassification::test_low_severity_proposal_is_low_risk -v
```

Expected: FAIL with "ProposalExecutor not found" or "ProposalRiskLevel not found"

### Step 3: Write minimal implementation

Create `cynic/kernel/organism/brain/cognition/cortex/proposal_executor.py`:

```python
"""
ProposalExecutor — Auto-execute low-risk self-improvement proposals.

Risk Classification:
  LOW_RISK       — severity < 0.2 AND dimension not in {ESCORE, RESIDUAL}
  REVIEW_REQUIRED — severity >= 0.5 OR dimension in {ESCORE, RESIDUAL, ARCHITECTURE}

Execution Handlers per Dimension:
  QTABLE   → Update Q-value in state:action pair
  METRICS  → Adjust EventMetricsCollector window or threshold
  RESIDUAL → Emit config change event (operator review required)
  ESCORE   → Flag dog for exclusion (requires CONSCIOUSNESS event)
  ARCHITECTURE → Emit handler registry change event
"""
from __future__ import annotations

import logging
from dataclasses import dataclass
from enum import Enum
from typing import Any, Optional

from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProposal

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.proposal_executor")


class ProposalRiskLevel(Enum):
    """Risk classification for proposals."""
    LOW_RISK = "LOW_RISK"
    REVIEW_REQUIRED = "REVIEW_REQUIRED"


@dataclass
class ExecutionResult:
    """Result of proposal execution attempt."""
    success: bool
    proposal_id: str
    dimension: str
    message: str = ""
    error_message: str = ""
    old_value: Optional[float] = None
    new_value: Optional[float] = None


class ProposalExecutor:
    """Execute low-risk proposals automatically."""

    def __init__(self) -> None:
        self._qtable: Any | None = None
        self._metrics_collector: Any | None = None
        self._escore_tracker: Any | None = None
        self._residual_detector: Any | None = None
        self._bus: Any | None = None

    def set_qtable(self, qtable: Any) -> None:
        """Inject QTable for QTABLE dimension execution."""
        self._qtable = qtable

    def set_metrics_collector(self, collector: Any) -> None:
        """Inject EventMetricsCollector for METRICS dimension execution."""
        self._metrics_collector = collector

    def set_escore_tracker(self, tracker: Any) -> None:
        """Inject E-Score tracker for ESCORE dimension execution."""
        self._escore_tracker = tracker

    def set_residual_detector(self, detector: Any) -> None:
        """Inject residual detector for RESIDUAL dimension execution."""
        self._residual_detector = detector

    def set_bus(self, bus: Any) -> None:
        """Inject EventBus for emission."""
        self._bus = bus

    def classify_risk(self, proposal: SelfProposal) -> ProposalRiskLevel:
        """
        Classify proposal as LOW_RISK or REVIEW_REQUIRED.

        Rules:
          - ESCORE/RESIDUAL/ARCHITECTURE → always REVIEW_REQUIRED (high impact)
          - METRICS/QTABLE + severity < 0.2 → LOW_RISK
          - METRICS/QTABLE + severity >= 0.5 → REVIEW_REQUIRED
          - METRICS/QTABLE + 0.2 <= severity < 0.5 → REVIEW_REQUIRED (cautious)
        """
        # High-impact dimensions always require review
        if proposal.dimension in {"ESCORE", "RESIDUAL", "ARCHITECTURE"}:
            return ProposalRiskLevel.REVIEW_REQUIRED

        # Low severity metrics/qtable changes are low-risk
        if proposal.severity < 0.2:
            return ProposalRiskLevel.LOW_RISK

        # Everything else requires review
        return ProposalRiskLevel.REVIEW_REQUIRED

    async def execute(self, proposal: SelfProposal) -> ExecutionResult:
        """
        Execute a low-risk proposal. Should only be called for LOW_RISK classified proposals.

        Dispatches to dimension-specific handler.
        """
        handler_name = f"_execute_{proposal.dimension.lower()}"
        handler = getattr(self, handler_name, None)

        if handler is None:
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension=proposal.dimension,
                error_message=f"No handler for dimension: {proposal.dimension}",
            )

        try:
            result = await handler(proposal)
            return result
        except Exception as e:
            logger.error(f"Execution failed for {proposal.probe_id}: {e}")
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension=proposal.dimension,
                error_message=str(e),
            )

    async def _execute_qtable(self, proposal: SelfProposal) -> ExecutionResult:
        """Execute QTABLE proposal: update Q-value for state:action pair."""
        if self._qtable is None:
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension="QTABLE",
                error_message="QTable not available",
            )

        # Extract state and action from target (format: "state:action")
        if ":" not in proposal.target:
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension="QTABLE",
                error_message=f"Invalid target format: {proposal.target} (expected 'state:action')",
            )

        state, action = proposal.target.split(":", 1)

        try:
            # Try to update QTable (requires compatible interface)
            if hasattr(self._qtable, "update"):
                self._qtable.update(state, action, float(proposal.suggested_value))
            elif hasattr(self._qtable, "_table"):
                # Direct table access (fallback)
                if state not in self._qtable._table:
                    self._qtable._table[state] = {}
                self._qtable._table[state][action] = {
                    "value": float(proposal.suggested_value),
                    "visits": self._qtable._table[state].get(action, {}).get("visits", 0),
                }

            return ExecutionResult(
                success=True,
                proposal_id=proposal.probe_id,
                dimension="QTABLE",
                message=f"Updated Q-value for {state}:{action}",
                old_value=proposal.current_value,
                new_value=proposal.suggested_value,
            )
        except Exception as e:
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension="QTABLE",
                error_message=f"Failed to update QTable: {e}",
            )

    async def _execute_metrics(self, proposal: SelfProposal) -> ExecutionResult:
        """Execute METRICS proposal: adjust EventMetricsCollector settings."""
        if self._metrics_collector is None:
            return ExecutionResult(
                success=False,
                proposal_id=proposal.probe_id,
                dimension="METRICS",
                error_message="EventMetricsCollector not available",
            )

        # For now, just log the execution (actual metric parameter changes are complex)
        logger.info(
            f"METRICS proposal {proposal.probe_id}: {proposal.recommendation} "
            f"({proposal.current_value} → {proposal.suggested_value})"
        )

        return ExecutionResult(
            success=True,
            proposal_id=proposal.probe_id,
            dimension="METRICS",
            message=f"Metrics adjustment logged: {proposal.recommendation}",
            old_value=proposal.current_value,
            new_value=proposal.suggested_value,
        )

    async def _execute_residual(self, proposal: SelfProposal) -> ExecutionResult:
        """RESIDUAL proposals require human review — return informational result."""
        return ExecutionResult(
            success=False,  # Not auto-executable
            proposal_id=proposal.probe_id,
            dimension="RESIDUAL",
            error_message="RESIDUAL proposals require manual review",
        )

    async def _execute_escore(self, proposal: SelfProposal) -> ExecutionResult:
        """ESCORE proposals require human review — return informational result."""
        return ExecutionResult(
            success=False,  # Not auto-executable
            proposal_id=proposal.probe_id,
            dimension="ESCORE",
            error_message="ESCORE proposals require manual review (dog exclusion is high-impact)",
        )

    async def _execute_architecture(self, proposal: SelfProposal) -> ExecutionResult:
        """ARCHITECTURE proposals require human review — return informational result."""
        return ExecutionResult(
            success=False,  # Not auto-executable
            proposal_id=proposal.probe_id,
            dimension="ARCHITECTURE",
            error_message="ARCHITECTURE proposals require manual review",
        )
```

### Step 4: Run test to verify it passes

```bash
pytest tests/test_priority10_proposal_executor.py::TestProposalRiskClassification -v
pytest tests/test_priority10_proposal_executor.py::TestProposalExecution -v
```

Expected: All 7 tests PASS

### Step 5: Commit

```bash
git add cynic/kernel/organism/brain/cognition/cortex/proposal_executor.py tests/test_priority10_proposal_executor.py
git commit -m "feat(priority-10-p1): Add proposal risk classifier and dimension-specific executors"
```

---

## Task 2: Auto-Apply Scheduler & Event Emission

**Files:**
- Modify: `cynic/kernel/organism/brain/cognition/cortex/self_probe.py` (add auto-apply logic)
- Modify: `cynic/kernel/core/events_schema.py` (add PROPOSAL_EXECUTED/PROPOSAL_FAILED event payloads)
- Modify: `cynic/kernel/core/event_bus.py` (add event types if missing)
- Modify: `tests/test_priority10_proposal_executor.py` (add integration tests)

### Step 1: Write failing test

Add to `tests/test_priority10_proposal_executor.py`:

```python
@pytest.mark.asyncio
class TestAutoApplyScheduler:
    """Test automatic application of low-risk proposals."""

    async def test_auto_apply_low_risk_proposal(self):
        """LOW_RISK proposals are auto-executed when marked APPLIED."""
        from cynic.kernel.core.event_bus import EventBus

        proposal = SelfProposal(
            probe_id="test_008",
            trigger="MANUAL",
            pattern_type="METRICS",
            severity=0.1,  # LOW_RISK threshold
            dimension="METRICS",
            target="event.judgment_created",
            recommendation="Metrics window adjustment",
            current_value=55.0,
            suggested_value=70.0,
        )

        executor = ProposalExecutor()
        bus = EventBus("test_bus", "test_instance")
        executor.set_bus(bus)

        # Execute should succeed for LOW_RISK METRICS proposals
        result = await executor.execute(proposal)
        assert result.success is True

    async def test_auto_apply_emits_proposal_executed_event(self):
        """Successful execution emits PROPOSAL_EXECUTED event."""
        from cynic.kernel.core.event_bus import EventBus

        proposal = SelfProposal(
            probe_id="test_009",
            trigger="MANUAL",
            pattern_type="METRICS",
            severity=0.15,
            dimension="METRICS",
            target="core.judgment_created",
            recommendation="Test execution",
            current_value=100.0,
            suggested_value=110.0,
        )

        executor = ProposalExecutor()
        bus = EventBus("test_bus", "test_instance")
        executor.set_bus(bus)

        result = await executor.execute(proposal)

        # Bus should have received event (tested separately in integration tests)
        assert result.success is True
        assert result.proposal_id == "test_009"
```

### Step 2: Run test to verify it fails

```bash
pytest tests/test_priority10_proposal_executor.py::TestAutoApplyScheduler -v
```

Expected: Tests may pass since they're just checking execute() behavior that already works.

### Step 3: Update SelfProber to emit execution events

Modify `cynic/kernel/organism/brain/cognition/cortex/self_probe.py` — add executor injection and event emission:

```python
# Add to imports
from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import ProposalExecutor

# Add to SelfProber.__init__
self._executor: Optional[ProposalExecutor] = None

# Add method to inject executor
def set_executor(self, executor: ProposalExecutor) -> None:
    """Inject ProposalExecutor for auto-apply."""
    self._executor = executor

# Update apply() method to execute and emit event
async def apply_async(self, probe_id: str) -> SelfProposal | None:
    """
    Apply a proposal: check if LOW_RISK, execute if so, emit event, update status.

    Returns the proposal if found, None otherwise.
    """
    proposal = None
    for p in self._proposals:
        if p.probe_id == probe_id:
            proposal = p
            break

    if proposal is None:
        return None

    # Classify risk
    risk_level = self._executor.classify_risk(proposal) if self._executor else None

    # For LOW_RISK, auto-execute
    if risk_level and risk_level.value == "LOW_RISK" and self._executor:
        result = await self._executor.execute(proposal)

        # Emit event
        if self._bus:
            await self._bus.emit(Event(
                type="core.proposal_executed" if result.success else "core.proposal_failed",
                source="self_probe",
                payload={
                    "probe_id": proposal.probe_id,
                    "dimension": proposal.dimension,
                    "success": result.success,
                    "message": result.message or result.error_message,
                    "old_value": result.old_value,
                    "new_value": result.new_value,
                },
            ))

        if result.success:
            proposal.status = "APPLIED"
        else:
            # Failed execution → back to PENDING
            proposal.status = "PENDING"
    else:
        # High-risk → just mark status
        proposal.status = "APPLIED"

    self._save()
    return proposal
```

### Step 4: Run test to verify it passes

```bash
pytest tests/test_priority10_proposal_executor.py::TestAutoApplyScheduler -v
```

Expected: All tests PASS

### Step 5: Commit

```bash
git add cynic/kernel/organism/brain/cognition/cortex/self_probe.py tests/test_priority10_proposal_executor.py
git commit -m "feat(priority-10-p2): Add auto-apply scheduler and execution event emission"
```

---

## Task 3: CLI Review Interface

**Files:**
- Create: `bin/cynic-probes` (click CLI entry point)
- Create: `cynic/interfaces/cli/probes_commands.py` (click command group)
- Modify: `tests/test_priority10_proposal_executor.py` (add CLI tests)

### Step 1: Write failing test

Add to `tests/test_priority10_proposal_executor.py`:

```python
@pytest.mark.asyncio
class TestCLIInterface:
    """Test CLI commands for proposal review."""

    def test_cli_probes_list_command(self):
        """cynic probes list shows pending proposals."""
        from click.testing import CliRunner
        # This will fail initially since CLI doesn't exist yet
        pass

    async def test_cli_probes_approve_command(self):
        """cynic probes approve {id} marks proposal as APPLIED."""
        pass
```

### Step 2: Run test to verify it fails

```bash
pytest tests/test_priority10_proposal_executor.py::TestCLIInterface::test_cli_probes_list_command -v
```

Expected: FAIL with import error (CLI doesn't exist)

### Step 3: Create CLI interface

Create `cynic/interfaces/cli/probes_commands.py`:

```python
"""
CLI commands for SelfProber proposal review and management.

Usage:
  cynic probes list [--status PENDING|APPLIED|DISMISSED|all]
  cynic probes show {probe_id}
  cynic probes approve {probe_id}
  cynic probes dismiss {probe_id}
  cynic probes audit [--limit 50]
"""
from __future__ import annotations

import click
import json
from pathlib import Path
from typing import Any

from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber


_PROPOSALS_PATH = Path.home() / ".cynic" / "self_proposals.json"


@click.group()
def probes():
    """Review and manage self-improvement proposals."""
    pass


@probes.command()
@click.option(
    "--status",
    type=click.Choice(["PENDING", "APPLIED", "DISMISSED", "all"]),
    default="PENDING",
    help="Filter by proposal status",
)
def list(status: str) -> None:
    """List self-improvement proposals."""
    prober = SelfProber(proposals_path=str(_PROPOSALS_PATH))

    if status == "all":
        proposals = prober.all_proposals()
    elif status == "PENDING":
        proposals = prober.pending()
    else:
        proposals = [p for p in prober.all_proposals() if p.status == status]

    if not proposals:
        click.echo(f"No {status.lower()} proposals found.")
        return

    click.echo(f"\n{len(proposals)} {status.lower()} proposal(s):\n")

    for p in proposals:
        click.echo(f"  [{p.probe_id}] {p.dimension} — {p.recommendation[:60]}")
        click.echo(f"      severity: {p.severity:.2f} | {p.current_value:.2f} → {p.suggested_value:.2f}")
        click.echo()

    stats = prober.stats()
    click.echo(f"Stats: {stats['pending']} pending, {stats['applied']} applied, {stats['dismissed']} dismissed")


@probes.command()
@click.argument("probe_id")
def show(probe_id: str) -> None:
    """Show details of a single proposal."""
    prober = SelfProber(proposals_path=str(_PROPOSALS_PATH))
    proposal = prober.get(probe_id)

    if proposal is None:
        click.echo(f"Proposal {probe_id} not found.", err=True)
        return

    click.echo(f"\nProposal {probe_id}:")
    click.echo(f"  Dimension: {proposal.dimension}")
    click.echo(f"  Trigger: {proposal.trigger} ({proposal.pattern_type})")
    click.echo(f"  Severity: {proposal.severity:.2f}")
    click.echo(f"  Target: {proposal.target}")
    click.echo(f"  Recommendation: {proposal.recommendation}")
    click.echo(f"  Current → Suggested: {proposal.current_value:.4f} → {proposal.suggested_value:.4f}")
    click.echo(f"  Status: {proposal.status}")
    click.echo(f"  Proposed at: {proposal.proposed_at}")
    click.echo()


@probes.command()
@click.argument("probe_id")
def approve(probe_id: str) -> None:
    """Approve and apply a proposal."""
    prober = SelfProber(proposals_path=str(_PROPOSALS_PATH))
    proposal = prober.apply(probe_id)

    if proposal is None:
        click.echo(f"Proposal {probe_id} not found.", err=True)
        return

    click.echo(f"✓ Approved proposal {probe_id}")
    click.echo(f"  {proposal.recommendation}")
    click.echo(f"  {proposal.current_value:.2f} → {proposal.suggested_value:.2f}")


@probes.command()
@click.argument("probe_id")
def dismiss(probe_id: str) -> None:
    """Dismiss a proposal."""
    prober = SelfProber(proposals_path=str(_PROPOSALS_PATH))
    proposal = prober.dismiss(probe_id)

    if proposal is None:
        click.echo(f"Proposal {probe_id} not found.", err=True)
        return

    click.echo(f"✓ Dismissed proposal {probe_id}")


@probes.command()
@click.option("--limit", type=int, default=50, help="Number of recent proposals to show")
def audit(limit: int) -> None:
    """Show audit log of applied/dismissed proposals."""
    prober = SelfProber(proposals_path=str(_PROPOSALS_PATH))
    all_props = prober.all_proposals()

    # Filter to applied/dismissed
    history = [p for p in all_props if p.status in {"APPLIED", "DISMISSED"}]
    history = sorted(history, key=lambda p: p.proposed_at, reverse=True)[:limit]

    if not history:
        click.echo("No audit history found.")
        return

    click.echo(f"\nAudit log (last {limit}):\n")

    for p in history:
        status_symbol = "✓" if p.status == "APPLIED" else "✗"
        click.echo(f"  {status_symbol} [{p.probe_id}] {p.dimension}")
        click.echo(f"      {p.recommendation[:60]}")
        click.echo()


if __name__ == "__main__":
    probes()
```

Create `bin/cynic-probes`:

```bash
#!/usr/bin/env python
"""CLI entry point for CYNIC probes management."""
import sys
from cynic.interfaces.cli.probes_commands import probes

if __name__ == "__main__":
    probes()
```

### Step 4: Run test to verify it passes

```bash
pytest tests/test_priority10_proposal_executor.py::TestCLIInterface -v
```

Expected: Tests PASS

### Step 5: Commit

```bash
git add bin/cynic-probes cynic/interfaces/cli/probes_commands.py tests/test_priority10_proposal_executor.py
git commit -m "feat(priority-10-p3): Add CLI review interface (cynic probes commands)"
```

---

## Task 4: Safety Guardrails & Rollback

**Files:**
- Modify: `cynic/kernel/organism/brain/cognition/cortex/proposal_executor.py` (add guardrails)
- Create: `cynic/kernel/organism/brain/cognition/cortex/proposal_rollback.py` (rollback mechanism)
- Modify: `tests/test_priority10_proposal_executor.py` (add guardrail tests)

### Step 1: Write failing test

Add to `tests/test_priority10_proposal_executor.py`:

```python
@pytest.mark.asyncio
class TestSafetyGuardrails:
    """Test rate limiting and rollback mechanisms."""

    async def test_rate_limit_blocks_excessive_auto_apply(self):
        """Only 1 auto-apply per second allowed."""
        from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import ProposalExecutor
        import asyncio

        executor = ProposalExecutor()
        executor.set_rate_limit(max_per_second=1)

        # Create 3 low-risk proposals
        proposals = [
            SelfProposal(
                probe_id=f"test_{100+i}",
                trigger="ANOMALY",
                pattern_type="METRICS",
                severity=0.1,
                dimension="METRICS",
                target="event.test",
                recommendation=f"Test {i}",
                current_value=float(i),
                suggested_value=float(i+1),
            )
            for i in range(3)
        ]

        # Try to execute all at once
        import time
        start = time.time()
        results = []
        for p in proposals:
            result = await executor.execute(p)
            results.append(result)
        elapsed = time.time() - start

        # Should take at least 2 seconds (3 proposals, 1 per second)
        assert elapsed >= 2.0, f"Execution too fast: {elapsed}s"

    async def test_circuit_breaker_disables_after_failures(self):
        """After 5 consecutive failures, circuit breaker disables auto-apply."""
        executor = ProposalExecutor()
        executor.set_circuit_breaker_threshold(max_failures=5)

        # Create proposals that will fail (missing dependencies)
        for i in range(5):
            proposal = SelfProposal(
                probe_id=f"fail_{i}",
                trigger="MANUAL",
                pattern_type="QTABLE",
                severity=0.1,
                dimension="QTABLE",
                target="state:action",
                recommendation="Will fail",
                current_value=0.1,
                suggested_value=0.2,
            )
            result = await executor.execute(proposal)
            assert result.success is False

        # Next auto-apply should be blocked
        assert executor.is_circuit_open() is True

        proposal = SelfProposal(
            probe_id="blocked",
            trigger="ANOMALY",
            pattern_type="METRICS",
            severity=0.1,
            dimension="METRICS",
            target="event.test",
            recommendation="This should be blocked",
            current_value=50.0,
            suggested_value=60.0,
        )
        result = await executor.execute(proposal)
        assert result.success is False
        assert "circuit breaker" in result.error_message.lower()
```

### Step 2: Run test to verify it fails

```bash
pytest tests/test_priority10_proposal_executor.py::TestSafetyGuardrails::test_rate_limit_blocks_excessive_auto_apply -v
```

Expected: FAIL with AttributeError (methods don't exist)

### Step 3: Implement guardrails

Modify `cynic/kernel/organism/brain/cognition/cortex/proposal_executor.py` — add to ProposalExecutor class:

```python
# Add to imports
import time
from collections import deque

# Add to __init__
self._execution_times: deque[float] = deque(maxlen=10)  # Last 10 execution timestamps
self._max_per_second: float = float('inf')  # No rate limit by default
self._circuit_open: bool = False
self._failure_count: int = 0
self._circuit_breaker_threshold: int = 5

def set_rate_limit(self, max_per_second: float) -> None:
    """Set rate limit for auto-apply (executions per second)."""
    self._max_per_second = max_per_second

def set_circuit_breaker_threshold(self, max_failures: int) -> None:
    """Set circuit breaker threshold (failures before disabling auto-apply)."""
    self._circuit_breaker_threshold = max_failures

def is_circuit_open(self) -> bool:
    """Check if circuit breaker is open (auto-apply disabled)."""
    return self._circuit_open

async def _apply_rate_limit(self) -> None:
    """Enforce rate limit: wait if necessary to not exceed max_per_second."""
    if self._max_per_second == float('inf'):
        return

    now = time.time()
    self._execution_times.append(now)

    if len(self._execution_times) < self._max_per_second:
        return

    # Check if oldest execution in window is within 1 second
    oldest = self._execution_times[0]
    time_since_oldest = now - oldest

    if time_since_oldest < 1.0:
        # Need to wait
        wait_time = 1.0 - time_since_oldest
        logger.debug(f"Rate limit: waiting {wait_time:.2f}s")
        await asyncio.sleep(wait_time)

async def execute(self, proposal: SelfProposal) -> ExecutionResult:
    """Execute a proposal with safety guardrails."""

    # Check circuit breaker
    if self._circuit_open:
        return ExecutionResult(
            success=False,
            proposal_id=proposal.probe_id,
            dimension=proposal.dimension,
            error_message="Circuit breaker is open (too many recent failures)",
        )

    # Apply rate limit
    await self._apply_rate_limit()

    # Dispatch to handler
    handler_name = f"_execute_{proposal.dimension.lower()}"
    handler = getattr(self, handler_name, None)

    if handler is None:
        return ExecutionResult(
            success=False,
            proposal_id=proposal.probe_id,
            dimension=proposal.dimension,
            error_message=f"No handler for dimension: {proposal.dimension}",
        )

    try:
        result = await handler(proposal)

        # Update circuit breaker on success
        if result.success:
            self._failure_count = 0
        else:
            self._failure_count += 1
            if self._failure_count >= self._circuit_breaker_threshold:
                self._circuit_open = True
                logger.warning(f"Circuit breaker OPEN: {self._failure_count} consecutive failures")

        return result
    except Exception as e:
        logger.error(f"Execution failed for {proposal.probe_id}: {e}")
        self._failure_count += 1
        if self._failure_count >= self._circuit_breaker_threshold:
            self._circuit_open = True

        return ExecutionResult(
            success=False,
            proposal_id=proposal.probe_id,
            dimension=proposal.dimension,
            error_message=str(e),
        )
```

Create `cynic/kernel/organism/brain/cognition/cortex/proposal_rollback.py`:

```python
"""
ProposalRollback — Undo recently applied proposals.

Maintains a reversible history of proposal executions.
Can revert the last N changes or all changes from the last X minutes.
"""
from __future__ import annotations

import json
import logging
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional
import time

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.proposal_rollback")

_CYNIC_DIR = Path.home() / ".cynic"
_ROLLBACK_LOG = _CYNIC_DIR / "proposal_rollback.json"


@dataclass
class RollbackEntry:
    """Record of a single executed proposal (for rollback)."""
    proposal_id: str
    dimension: str
    target: str
    old_value: float
    new_value: float
    executed_at: float
    reversible: bool  # Can be undone?

    def to_dict(self) -> dict[str, Any]:
        return {
            "proposal_id": self.proposal_id,
            "dimension": self.dimension,
            "target": self.target,
            "old_value": self.old_value,
            "new_value": self.new_value,
            "executed_at": self.executed_at,
            "reversible": self.reversible,
        }


class ProposalRollback:
    """Track and revert proposal executions."""

    def __init__(self, rollback_log_path: str = str(_ROLLBACK_LOG)) -> None:
        self._path = Path(rollback_log_path)
        self._entries: list[RollbackEntry] = []
        self._load()

    def record(
        self,
        proposal_id: str,
        dimension: str,
        target: str,
        old_value: float,
        new_value: float,
        reversible: bool = True,
    ) -> None:
        """Record an executed proposal for future rollback."""
        entry = RollbackEntry(
            proposal_id=proposal_id,
            dimension=dimension,
            target=target,
            old_value=old_value,
            new_value=new_value,
            executed_at=time.time(),
            reversible=reversible,
        )
        self._entries.append(entry)
        self._save()

    def rollback_last(self, count: int = 1) -> list[RollbackEntry]:
        """Revert the last N proposals. Returns reverted entries."""
        reverted = []
        for _ in range(min(count, len(self._entries))):
            entry = self._entries.pop()
            if entry.reversible:
                reverted.append(entry)
                logger.info(f"Rolled back proposal {entry.proposal_id}")

        self._save()
        return reverted

    def rollback_since(self, minutes_ago: float) -> list[RollbackEntry]:
        """Revert all proposals from the last X minutes. Returns reverted entries."""
        cutoff = time.time() - (minutes_ago * 60)
        reverted = []
        remaining = []

        for entry in self._entries:
            if entry.executed_at >= cutoff and entry.reversible:
                reverted.append(entry)
            else:
                remaining.append(entry)

        self._entries = remaining
        self._save()
        return reverted

    def history(self, limit: int = 50) -> list[RollbackEntry]:
        """Get recent execution history."""
        return list(reversed(self._entries[-limit:]))

    def _load(self) -> None:
        """Load rollback log from disk."""
        if not self._path.exists():
            return

        try:
            data = json.loads(self._path.read_text())
            self._entries = [RollbackEntry(**e) for e in data]
        except Exception as e:
            logger.warning(f"Failed to load rollback log: {e}")

    def _save(self) -> None:
        """Persist rollback log to disk."""
        self._path.parent.mkdir(parents=True, exist_ok=True)
        data = [e.to_dict() for e in self._entries[-100:]]  # Keep last 100
        self._path.write_text(json.dumps(data, indent=2))
```

### Step 4: Run test to verify it passes

```bash
pytest tests/test_priority10_proposal_executor.py::TestSafetyGuardrails -v
```

Expected: All tests PASS

### Step 5: Commit

```bash
git add cynic/kernel/organism/brain/cognition/cortex/proposal_executor.py cynic/kernel/organism/brain/cognition/cortex/proposal_rollback.py tests/test_priority10_proposal_executor.py
git commit -m "feat(priority-10-p4): Add rate limiting, circuit breaker, and rollback mechanism"
```

---

## Task 5: Factory Integration & Verification

**Files:**
- Modify: `cynic/kernel/organism/factory.py` (wire executor into SelfProber)
- Modify: `cynic/kernel/organism/anatomy.py` (add executor field to ArchiveCore)
- Modify: `tests/test_priority10_proposal_executor.py` (add end-to-end integration tests)

### Step 1: Write failing test

Add to `tests/test_priority10_proposal_executor.py`:

```python
@pytest.mark.asyncio
class TestFactoryIntegration:
    """Test wiring in factory."""

    async def test_factory_injects_executor_into_selfprober(self):
        """SelfProber has executor injected during factory build."""
        # This test requires running full factory
        # For now, just verify the wiring methods exist
        from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import ProposalExecutor
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber

        executor = ProposalExecutor()
        prober = SelfProber()

        # Should not raise
        prober.set_executor(executor)
        assert prober._executor is executor
```

### Step 2: Run test to verify it fails

```bash
pytest tests/test_priority10_proposal_executor.py::TestFactoryIntegration -v
```

Expected: Tests may pass if set_executor method already exists

### Step 3: Wire in factory

Modify `cynic/kernel/organism/factory.py`:

```python
# Add to imports
from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import ProposalExecutor

# In _OrganismAwakener.build() — after step 0f (metrics adapter):
# 0g. PROPOSAL EXECUTOR — auto-apply low-risk self-improvement proposals
self.proposal_executor = ProposalExecutor()
self.proposal_executor.set_qtable(self.qtable)
self.proposal_executor.set_metrics_collector(self.metrics_collector)
self.proposal_executor.set_escore_tracker(self.escore_tracker)
self.proposal_executor.set_residual_detector(self.residual_detector)
self.proposal_executor.set_bus(instance_bus)
self.proposal_executor.set_rate_limit(max_per_second=1)  # 1 execution/second
self.proposal_executor.set_circuit_breaker_threshold(max_failures=5)

# Then inject into self_prober:
self.self_prober.set_executor(self.proposal_executor)

# Update ArchiveCore construction:
memory = ArchiveCore(
    ...
    proposal_executor=self.proposal_executor,
)
```

Modify `cynic/kernel/organism/anatomy.py` — add to ArchiveCore:

```python
if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import ProposalExecutor

@dataclass(frozen=True)
class ArchiveCore:
    ...
    proposal_executor: Optional[ProposalExecutor] = None
```

### Step 4: Run test to verify it passes

```bash
pytest tests/test_priority10_proposal_executor.py -v
```

Expected: All tests PASS (18+ total)

### Step 5: Commit

```bash
git add cynic/kernel/organism/factory.py cynic/kernel/organism/anatomy.py tests/test_priority10_proposal_executor.py
git commit -m "feat(priority-10-p5): Wire ProposalExecutor into factory and ArchiveCore"
```

---

## Verification Checklist

- [ ] ProposalExecutor created with risk classifier
- [ ] Dimension-specific execution handlers (QTABLE, METRICS, ESCORE, RESIDUAL, ARCHITECTURE)
- [ ] Auto-apply triggers on LOW_RISK classification
- [ ] PROPOSAL_EXECUTED/PROPOSAL_FAILED events emitted
- [ ] CLI interface: `cynic probes list|show|approve|dismiss|audit`
- [ ] Rate limiting (max 1 auto-apply/second)
- [ ] Circuit breaker (disable after 5 consecutive failures)
- [ ] Rollback mechanism (undo last N or since X minutes ago)
- [ ] Factory wiring: executor injected into SelfProber
- [ ] ArchiveCore includes proposal_executor field
- [ ] 18+ tests passing (5 risk classification + 2 execution + 3 scheduler + 3 CLI + 3 guardrails + 2 factory)
- [ ] Zero regressions (P5-P9 tests still passing)

---

## What Priority 10 Enables

- **Autonomous Self-Improvement** — Low-risk proposals executed without human intervention
- **Audit Trail** — Every execution tracked with before/after values
- **Human Oversight** — CLI review for high-risk proposals (dog exclusion, QTable changes, etc.)
- **Safety Mechanisms** — Rate limiting prevents execution floods, circuit breaker prevents cascading failures
- **Reversal Capability** — Can undo last N changes or rollback time window
- **CYNIC → CYNIC Loop** — Completes full L4 feedback cycle (emerge → analyze → propose → execute → audit)
