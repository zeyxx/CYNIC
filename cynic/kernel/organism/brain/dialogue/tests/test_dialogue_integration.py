"""Integration tests for dialogue + learning + classification working together."""

import pytest
from cynic.kernel.organism.brain.dialogue.models import UserMessage, CynicMessage
from cynic.kernel.organism.brain.dialogue.storage import DialogueStore
from cynic.kernel.organism.brain.learning.memory_store import MemoryStore
from cynic.kernel.organism.brain.collaborative.decision_classifier import DecisionClassifier, DecisionClass


@pytest.mark.asyncio
async def test_full_dialogue_workflow(tmp_path):
    """Full workflow: user question → reasoning → response → storage."""
    # Initialize storage
    storage = DialogueStore(tmp_path / "dialogue.db")
    await storage.initialize()

    # Create and save user message
    user_msg = UserMessage(
        "question", "Why did you choose WAG?", 0.5, "j123"
    )
    msg_id_1 = await storage.save_message(user_msg)
    assert msg_id_1 is not None

    # Create and save CYNIC response
    cynic_msg = CynicMessage(
        "reasoning",
        "I chose WAG because PHI (0.8) and CULTURE (0.75) suggested balance",
        0.5,
        {"PHI": 0.8, "CULTURE": 0.75},
        "j123"
    )
    msg_id_2 = await storage.save_message(cynic_msg)
    assert msg_id_2 is not None

    # Retrieve conversation context
    context = await storage.get_conversation_context("j123")
    assert len(context) == 2
    assert context[0]["is_user"] == 1
    assert context[1]["is_user"] == 0


@pytest.mark.asyncio
async def test_learning_from_feedback(tmp_path):
    """Update memory based on user feedback."""
    memory_store = MemoryStore(tmp_path)
    await memory_store.initialize()

    # Load initial memory
    initial = await memory_store.load_memory()
    assert initial.user_values["PHI"] == 0.5

    # User feedback: "I prefer more caution"
    # Update BURN axiom weight
    updated = initial.update_value("BURN", 0.8)

    # Save updated memory
    await memory_store.save_memory(updated)

    # Reload and verify
    reloaded = await memory_store.load_memory()
    assert reloaded.user_values["BURN"] == 0.8


@pytest.mark.asyncio
async def test_decision_classification_learning():
    """Learn decision types from approval patterns."""
    classifier = DecisionClassifier()

    decision = {"action": "compress_traces"}

    # Record approvals
    for _ in range(3):
        classifier.learn_approval(decision, DecisionClass.AUTONOMOUS)

    # Should now be autonomous
    classification = classifier.classify(decision)
    assert classification == DecisionClass.AUTONOMOUS

    # Check statistics
    stats = classifier.get_statistics()
    assert stats["autonomous_count"] >= 1


@pytest.mark.asyncio
async def test_dialogue_with_persistence(tmp_path):
    """Dialogue messages persist across sessions."""
    storage = DialogueStore(tmp_path / "dialogue.db")
    await storage.initialize()

    # Session 1: Add messages
    msg1 = UserMessage("question", "First question?", 0.5, None)
    msg2 = UserMessage("feedback", "That was helpful", 0.6, None)

    await storage.save_message(msg1)
    await storage.save_message(msg2)

    # Session 2: Load from persistent storage
    history = await storage.get_last_n_messages(10)
    assert len(history) == 2
    assert history[0]["message_type"] == "question"
    assert history[1]["message_type"] == "feedback"


@pytest.mark.asyncio
async def test_dialogue_axiom_tracking(tmp_path):
    """CYNIC messages track axiom scores across dialogue."""
    storage = DialogueStore(tmp_path / "dialogue.db")
    await storage.initialize()

    # Multiple CYNIC responses tracking different axioms
    msg1 = CynicMessage(
        "reasoning",
        "Prioritizing harmony",
        0.4,
        {"PHI": 0.9, "CULTURE": 0.7},
        "j456"
    )
    msg2 = CynicMessage(
        "reasoning",
        "Considering skepticism",
        0.45,
        {"VERIFY": 0.85, "BURN": 0.6},
        "j456"
    )

    await storage.save_message(msg1)
    await storage.save_message(msg2)

    # Retrieve and verify axiom data is persisted
    context = await storage.get_conversation_context("j456")
    assert len(context) == 2
    assert "axiom_scores" in context[0]
    assert "axiom_scores" in context[1]


@pytest.mark.asyncio
async def test_memory_learning_rate_adjustment(tmp_path):
    """Memory learning rate can be adjusted based on feedback."""
    memory_store = MemoryStore(tmp_path)
    await memory_store.initialize()

    # Load and adjust learning rate
    initial = await memory_store.load_memory()
    assert initial.learning_rate == 0.01

    # User prefers faster learning
    adjusted = initial.with_updated_learning_rate(0.05)
    await memory_store.save_memory(adjusted)

    # Verify persistence
    reloaded = await memory_store.load_memory()
    assert reloaded.learning_rate == 0.05


@pytest.mark.asyncio
async def test_decision_rejection_handling():
    """Classifier tracks rejections and keeps decisions as consultation."""
    classifier = DecisionClassifier()

    decision = {"action": "experimental_feature"}

    # Record rejections
    classifier.learn_rejection(decision)
    classifier.learn_rejection(decision)

    # Should remain CONSULTATION after rejections
    classification = classifier.classify(decision)
    assert classification == DecisionClass.CONSULTATION


@pytest.mark.asyncio
async def test_full_integration_dialogue_to_memory(tmp_path):
    """Complete flow: dialogue → feedback → memory update → learning rate change."""
    # Setup
    storage = DialogueStore(tmp_path / "dialogue.db")
    await storage.initialize()

    memory_store = MemoryStore(tmp_path)
    await memory_store.initialize()

    # Step 1: User asks a question
    user_question = UserMessage(
        "question",
        "Should we prioritize caution or harmony?",
        0.3,
        "j789"
    )
    await storage.save_message(user_question)

    # Step 2: CYNIC responds with reasoning
    cynic_response = CynicMessage(
        "reasoning",
        "Based on your values, harmony is slightly preferred",
        0.5,
        {"PHI": 0.7, "BURN": 0.4},
        "j789"
    )
    await storage.save_message(cynic_response)

    # Step 3: User provides feedback
    user_feedback = UserMessage(
        "feedback",
        "I actually prefer more caution in this case",
        0.4,
        "j789"
    )
    await storage.save_message(user_feedback)

    # Step 4: Update memory based on feedback
    memory = await memory_store.load_memory()
    updated_memory = memory.update_value("BURN", 0.7)  # Increase caution
    await memory_store.save_memory(updated_memory)

    # Step 5: Verify full conversation is persisted
    full_context = await storage.get_conversation_context("j789")
    assert len(full_context) == 3
    assert full_context[0]["message_type"] == "question"
    assert full_context[1]["message_type"] == "reasoning"
    assert full_context[2]["message_type"] == "feedback"

    # Step 6: Verify memory update persisted
    reloaded_memory = await memory_store.load_memory()
    assert reloaded_memory.user_values["BURN"] == 0.7


@pytest.mark.asyncio
async def test_multiple_judgment_conversations(tmp_path):
    """Multiple concurrent conversations with different judgment IDs."""
    storage = DialogueStore(tmp_path / "dialogue.db")
    await storage.initialize()

    # Conversation 1
    msg1a = UserMessage("question", "First judgment question?", 0.5, "j_a")
    msg1b = CynicMessage("reasoning", "My reasoning...", 0.5, {}, "j_a")
    await storage.save_message(msg1a)
    await storage.save_message(msg1b)

    # Conversation 2
    msg2a = UserMessage("question", "Second judgment question?", 0.4, "j_b")
    msg2b = CynicMessage("reasoning", "Different reasoning...", 0.45, {}, "j_b")
    await storage.save_message(msg2a)
    await storage.save_message(msg2b)

    # Verify conversations are isolated
    context_a = await storage.get_conversation_context("j_a")
    context_b = await storage.get_conversation_context("j_b")

    assert len(context_a) == 2
    assert len(context_b) == 2
    assert context_a[0]["related_judgment_id"] == "j_a"
    assert context_b[0]["related_judgment_id"] == "j_b"


@pytest.mark.asyncio
async def test_classifier_always_autonomous_decisions():
    """Pre-classified autonomous decisions don't need learning."""
    classifier = DecisionClassifier()

    # These are always autonomous
    for action in ["compress_traces", "cleanup_disk", "backup_experiment_log"]:
        decision = {"action": action}
        classification = classifier.classify(decision)
        assert classification == DecisionClass.AUTONOMOUS

    # No learning needed - immediately recognized
    stats = classifier.get_statistics()
    assert stats["total_learned"] == 0  # Not in learned_classes


@pytest.mark.asyncio
async def test_classifier_always_consultation_decisions():
    """Pre-classified consultation decisions."""
    classifier = DecisionClassifier()

    for action in ["change_axiom_weight", "modify_relationship_memory"]:
        decision = {"action": action}
        classification = classifier.classify(decision)
        assert classification == DecisionClass.CONSULTATION


@pytest.mark.asyncio
async def test_communication_style_inference(tmp_path):
    """Infer communication preferences from user messages."""
    memory_store = MemoryStore(tmp_path)
    await memory_store.initialize()

    memory = await memory_store.load_memory()

    # Create messages with different lengths
    short_messages = [
        UserMessage("question", "Short?", 0.5, None),
        UserMessage("feedback", "Yes.", 0.4, None),
        UserMessage("question", "OK?", 0.3, None),
    ]

    inferred = memory.infer_communication_style(short_messages)
    assert inferred["verbosity"] == "concise"

    # Long messages (average > 100 chars)
    long_messages = [
        UserMessage("question", "This is a very long detailed question about something important and complex that requires extensive explanation and discussion", 0.5, None),
        UserMessage("feedback", "I really appreciate the thorough analysis and detailed explanation of your comprehensive reasoning and thinking process", 0.5, None),
    ]

    inferred_long = memory.infer_communication_style(long_messages)
    assert inferred_long["verbosity"] == "detailed"
