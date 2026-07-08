from __future__ import annotations

from abc import ABC, abstractmethod

from .entities import AgentExposureGraph


class ExposureSourcePort(ABC):
    @abstractmethod
    def load(self) -> AgentExposureGraph:
        """Load the organism exposure graph."""
        raise NotImplementedError


class AgentProjectionPort(ABC):
    @abstractmethod
    def render(self, graph: AgentExposureGraph, agent: str) -> str:
        """Render an agent-specific projection from the exposure graph."""
        raise NotImplementedError
