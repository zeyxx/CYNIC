"""
CYNIC StorageInterface Compliance Tests — Phase 1

Verifies both SurrealStorage and PostgreSQL repos implement StorageInterface.
"""
from __future__ import annotations

import inspect

import pytest

from cynic.core.storage.interface import (
    StorageInterface,
    JudgmentRepoInterface,
    QTableRepoInterface,
    LearningRepoInterface,
    BenchmarkRepoInterface,
    ResidualRepoInterface,
    SDKSessionRepoInterface,
    ScholarRepoInterface,
    ActionProposalRepoInterface,
    DogSoulRepoInterface,
)


class TestSurrealImplementsInterface:
    """SurrealStorage and its repos must implement all interface methods."""

    def test_surreal_storage_is_storage_interface(self):
        from cynic.core.storage.surreal import SurrealStorage
        assert issubclass(SurrealStorage, StorageInterface)

    def test_judgment_repo_implements_interface(self):
        from cynic.core.storage.surreal import JudgmentRepo
        assert issubclass(JudgmentRepo, JudgmentRepoInterface)
        _check_methods(JudgmentRepoInterface, JudgmentRepo)

    def test_qtable_repo_implements_interface(self):
        from cynic.core.storage.surreal import QTableRepo
        assert issubclass(QTableRepo, QTableRepoInterface)
        _check_methods(QTableRepoInterface, QTableRepo)

    def test_learning_repo_implements_interface(self):
        from cynic.core.storage.surreal import LearningRepo
        assert issubclass(LearningRepo, LearningRepoInterface)
        _check_methods(LearningRepoInterface, LearningRepo)

    def test_benchmark_repo_implements_interface(self):
        from cynic.core.storage.surreal import BenchmarkRepo
        assert issubclass(BenchmarkRepo, BenchmarkRepoInterface)
        _check_methods(BenchmarkRepoInterface, BenchmarkRepo)

    def test_residual_repo_implements_interface(self):
        from cynic.core.storage.surreal import ResidualRepo
        assert issubclass(ResidualRepo, ResidualRepoInterface)
        _check_methods(ResidualRepoInterface, ResidualRepo)

    def test_sdk_session_repo_implements_interface(self):
        from cynic.core.storage.surreal import SDKSessionRepo
        assert issubclass(SDKSessionRepo, SDKSessionRepoInterface)
        _check_methods(SDKSessionRepoInterface, SDKSessionRepo)

    def test_scholar_repo_implements_interface(self):
        from cynic.core.storage.surreal import ScholarRepo
        assert issubclass(ScholarRepo, ScholarRepoInterface)
        _check_methods(ScholarRepoInterface, ScholarRepo)

    def test_action_proposal_repo_implements_interface(self):
        from cynic.core.storage.surreal import ActionProposalRepo
        assert issubclass(ActionProposalRepo, ActionProposalRepoInterface)
        _check_methods(ActionProposalRepoInterface, ActionProposalRepo)

    def test_dog_soul_repo_implements_interface(self):
        from cynic.core.storage.surreal import DogSoulRepo
        assert issubclass(DogSoulRepo, DogSoulRepoInterface)
        _check_methods(DogSoulRepoInterface, DogSoulRepo)


class TestPostgresImplementsInterface:
    """PostgreSQL repos must implement their respective interface methods."""

    def test_judgment_repo_implements_interface(self):
        from cynic.core.storage.postgres import JudgmentRepository
        assert issubclass(JudgmentRepository, JudgmentRepoInterface)
        _check_methods(JudgmentRepoInterface, JudgmentRepository)

    def test_qtable_repo_implements_interface(self):
        from cynic.core.storage.postgres import QTableRepository
        assert issubclass(QTableRepository, QTableRepoInterface)
        _check_methods(QTableRepoInterface, QTableRepository)

    def test_learning_repo_implements_interface(self):
        from cynic.core.storage.postgres import LearningRepository
        assert issubclass(LearningRepository, LearningRepoInterface)
        _check_methods(LearningRepoInterface, LearningRepository)

    def test_benchmark_repo_implements_interface(self):
        from cynic.core.storage.postgres import BenchmarkRepository
        assert issubclass(BenchmarkRepository, BenchmarkRepoInterface)
        _check_methods(BenchmarkRepoInterface, BenchmarkRepository)

    def test_residual_repo_implements_interface(self):
        from cynic.core.storage.postgres import ResidualRepository
        assert issubclass(ResidualRepository, ResidualRepoInterface)
        _check_methods(ResidualRepoInterface, ResidualRepository)

    def test_sdk_session_repo_implements_interface(self):
        from cynic.core.storage.postgres import SDKSessionRepository
        assert issubclass(SDKSessionRepository, SDKSessionRepoInterface)
        _check_methods(SDKSessionRepoInterface, SDKSessionRepository)

    def test_scholar_repo_implements_interface(self):
        from cynic.core.storage.postgres import ScholarRepository
        assert issubclass(ScholarRepository, ScholarRepoInterface)
        _check_methods(ScholarRepoInterface, ScholarRepository)


def _check_methods(interface: type, implementation: type) -> None:
    """Verify implementation has all abstract methods from interface."""
    for name, method in inspect.getmembers(interface, predicate=inspect.isfunction):
        if name.startswith("_"):
            continue
        assert hasattr(implementation, name), (
            f"{implementation.__name__} missing method {name} "
            f"from {interface.__name__}"
        )
