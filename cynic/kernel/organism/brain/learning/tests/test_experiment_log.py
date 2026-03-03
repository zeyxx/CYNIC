import tempfile
from pathlib import Path

import pytest

from cynic.kernel.organism.brain.learning.experiment_log import (
    Experiment,
    ExperimentLog,
)


def test_experiment_creation():
    """Create experiment with hypothesis and approach."""
    exp = Experiment(
        hypothesis="Dog 7 + Dog 11 produces better fairness",
        approach=["dogs: [7, 11]", "weights: {BURN: 0.8, CULTURE: 0.7}"],
        results={
            "user_satisfaction": 0.85,
            "q_score_accuracy": 0.78,
            "fairness_metric": 0.91,
        },
        status="successful",
        iterations=1,
    )

    assert exp.hypothesis == "Dog 7 + Dog 11 produces better fairness"
    assert exp.status == "successful"
    assert exp.results["user_satisfaction"] == 0.85
    assert exp.is_immutable


def test_experiment_immutability():
    """Experiments are frozen (immutable)."""
    exp = Experiment(
        hypothesis="Test", approach=[], results={}, status="successful", iterations=1
    )

    with pytest.raises((AttributeError, TypeError)):
        exp.hypothesis = "Modified"


@pytest.mark.asyncio
async def test_experiment_log_append():
    """Append experiment to log."""
    with tempfile.TemporaryDirectory() as tmpdir:
        log = ExperimentLog(Path(tmpdir))

        exp = Experiment(
            hypothesis="Test hypothesis",
            approach=["approach 1"],
            results={"score": 0.9},
            status="successful",
            iterations=1,
        )

        exp_id = await log.append(exp)
        assert exp_id is not None


@pytest.mark.asyncio
async def test_experiment_log_query():
    """Query successful experiments."""
    with tempfile.TemporaryDirectory() as tmpdir:
        log = ExperimentLog(Path(tmpdir))

        # Add two experiments
        exp1 = Experiment("Hyp 1", ["app1"], {"score": 0.9}, "successful", 1)
        exp2 = Experiment("Hyp 2", ["app2"], {"score": 0.5}, "failed", 1)

        await log.append(exp1)
        await log.append(exp2)

        # Query successful
        successful = await log.get_experiments_by_status("successful")
        assert len(successful) == 1


@pytest.mark.asyncio
async def test_experiment_log_all():
    """Get all experiments."""
    with tempfile.TemporaryDirectory() as tmpdir:
        log = ExperimentLog(Path(tmpdir))

        exp = Experiment("Test", [], {}, "successful", 1)
        await log.append(exp)

        all_exp = await log.get_all()
        assert len(all_exp) > 0
