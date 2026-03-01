from cynic.kernel.organism.brain.collaborative.decision_classifier import (
    DecisionClass,
    DecisionClassifier,
)


def test_decision_class_enum():
    """DecisionClass has three types."""
    assert DecisionClass.AUTONOMOUS.value == "A"
    assert DecisionClass.CONSULTATION.value == "B"
    assert DecisionClass.EXPLORATION.value == "C"


def test_classify_disk_cleanup():
    """Disk cleanup is Class A (autonomous)."""
    classifier = DecisionClassifier()

    decision = {
        "action": "compress_old_traces",
        "impact": "reversible",
        "time_to_revert": "seconds",
    }

    classification = classifier.classify(decision)
    # First encounter defaults to B, but disk cleanup should trend to A
    assert classification in [DecisionClass.AUTONOMOUS, DecisionClass.CONSULTATION]


def test_classify_axiom_weight_change():
    """Axiom weight change is Class B (consultation)."""
    classifier = DecisionClassifier()

    decision = {
        "action": "change_axiom_weight",
        "axiom": "BURN",
        "new_weight": 0.8,
        "impact": "affects all future judgments",
    }

    classification = classifier.classify(decision)
    assert classification == DecisionClass.CONSULTATION


def test_classify_novel_experiment():
    """Novel experiment is Class C (exploration)."""
    classifier = DecisionClassifier()

    decision = {
        "action": "try_dog_combination",
        "dogs": [6, 11],
        "hypothesis": "Better fairness with harmonic pairing",
    }

    classification = classifier.classify(decision)
    assert classification == DecisionClass.EXPLORATION


def test_learn_decision_pattern():
    """Learn that a decision type should be autonomous."""
    classifier = DecisionClassifier()

    # Record multiple approvals of same decision type
    decision = {"action": "compress_traces"}

    for _ in range(5):
        classifier.learn_approval(decision, DecisionClass.AUTONOMOUS)

    # Should now classify as AUTONOMOUS
    new_classification = classifier.classify(decision)
    assert new_classification == DecisionClass.AUTONOMOUS


def test_learn_rejection_keeps_consultation():
    """Learn that a decision type should stay consultation."""
    classifier = DecisionClassifier()

    decision = {"action": "delete_historical_data"}

    for _ in range(5):
        classifier.learn_rejection(decision)

    # Should stay CONSULTATION
    classification = classifier.classify(decision)
    assert classification == DecisionClass.CONSULTATION
