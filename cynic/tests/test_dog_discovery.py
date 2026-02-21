"""
CYNIC Dog Auto-Discovery Tests — Phase 7

Verifies that all 11 dogs are discoverable via pkgutil scanning.
"""
from __future__ import annotations

import pytest

from cynic.cognition.neurons.base import AbstractDog, DogId, LLMDog
from cynic.cognition.neurons.discovery import discover_dog_classes, discover_dogs


class TestDiscoverDogClasses:
    """discover_dog_classes() should find all 11 dogs."""

    def test_finds_all_11_dogs(self):
        """All 11 DogId members should be discovered."""
        classes = discover_dog_classes()
        assert len(classes) == 11
        for dog_id in DogId:
            assert dog_id in classes, f"Missing dog class for {dog_id}"

    def test_all_classes_are_abstract_dog_subclasses(self):
        """Every discovered class must be an AbstractDog subclass."""
        classes = discover_dog_classes()
        for dog_id, cls in classes.items():
            assert issubclass(cls, AbstractDog), (
                f"{cls.__name__} is not an AbstractDog subclass"
            )

    def test_dog_id_matches_class_attribute(self):
        """Each class's DOG_ID must match its key in the returned dict."""
        classes = discover_dog_classes()
        for dog_id, cls in classes.items():
            assert cls.DOG_ID == dog_id, (
                f"{cls.__name__}.DOG_ID = {cls.DOG_ID}, expected {dog_id}"
            )

    def test_no_abstract_classes_returned(self):
        """AbstractDog and LLMDog themselves should NOT be returned."""
        classes = discover_dog_classes()
        for cls in classes.values():
            assert cls is not AbstractDog
            assert cls is not LLMDog

    def test_llm_dogs_identified(self):
        """5 dogs should be LLMDog subclasses."""
        classes = discover_dog_classes()
        llm_dogs = {k: v for k, v in classes.items() if issubclass(v, LLMDog)}
        expected_llm = {DogId.SAGE, DogId.SCHOLAR, DogId.CARTOGRAPHER, DogId.DEPLOYER, DogId.SCOUT}
        assert set(llm_dogs.keys()) == expected_llm

    def test_non_llm_dogs_identified(self):
        """6 dogs should be direct AbstractDog subclasses (not LLMDog)."""
        classes = discover_dog_classes()
        non_llm = {k: v for k, v in classes.items() if not issubclass(v, LLMDog)}
        expected = {DogId.CYNIC, DogId.ANALYST, DogId.GUARDIAN, DogId.ORACLE, DogId.ARCHITECT, DogId.JANITOR}
        assert set(non_llm.keys()) == expected


class TestDiscoverDogs:
    """discover_dogs() should instantiate all dogs."""

    def test_instantiates_all_simple_dogs(self):
        """Dogs with no required args should auto-instantiate."""
        dogs = discover_dogs()
        # OracleDog accepts qtable=None by default, so all 11 should work
        assert len(dogs) == 11

    def test_override_oracle(self):
        """Override should replace auto-instantiation."""
        from cynic.cognition.neurons.oracle import OracleDog
        from cynic.learning.qlearning import QTable

        qt = QTable()
        custom_oracle = OracleDog(qtable=qt)
        dogs = discover_dogs(ORACLE=custom_oracle)
        assert dogs[DogId.ORACLE] is custom_oracle
        assert dogs[DogId.ORACLE]._qtable is qt

    def test_instances_have_correct_dog_id(self):
        """Each instance's dog_id should match its DogId key."""
        dogs = discover_dogs()
        for dog_id, dog in dogs.items():
            assert dog.dog_id == dog_id, (
                f"Instance dog_id={dog.dog_id}, expected {dog_id}"
            )

    def test_all_are_abstract_dog_instances(self):
        """All returned values must be AbstractDog instances."""
        dogs = discover_dogs()
        for dog in dogs.values():
            assert isinstance(dog, AbstractDog)

    def test_idempotent(self):
        """Multiple calls should return fresh instances each time."""
        dogs1 = discover_dogs()
        dogs2 = discover_dogs()
        for dog_id in dogs1:
            assert dogs1[dog_id] is not dogs2[dog_id], (
                f"{dog_id} returned same instance across calls"
            )
