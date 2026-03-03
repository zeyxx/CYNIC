"""
CYNIC Temporal MCTS Scientist - Hypothesis Tree Search.

Transforms Claude Code's "ai-co-scientist" concept into a secure, native MCTS protocol.
Instead of arbitrary shell scripts, experiments are nodes in a fractal tree,
evaluated via phi-bounded UCT (Upper Confidence Bound applied to Trees).

Lentille: ML Platform / Robotics
"""
import logging
import math
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from cynic.kernel.core.phi import PHI, PHI_INV

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.mcts_scientist")

@dataclass
class Hypothesis:
    """A falsifiable scientific statement about the codebase."""
    id: str
    description: str
    target_metric: str
    expected_trend: str  # "increase" or "decrease"
    mutations: Dict[str, str] = field(default_factory=dict) # filepath -> new_content

@dataclass
class ExperimentNode:
    """A node in the Temporal MCTS tree representing an experiment state."""
    id: str
    parent_id: Optional[str]
    hypothesis: Hypothesis
    visits: int = 0
    total_q_score: float = 0.0
    status: str = "PENDING"  # PENDING, RUNNING, SUCCESS, FAILED
    actual_q_score: float = 0.0
    children: List['ExperimentNode'] = field(default_factory=list)
    created_at: float = field(default_factory=time.time)

    @property
    def exploitation_score(self) -> float:
        if self.visits == 0:
            return 0.0
        return self.total_q_score / self.visits

    def uct_score(self, parent_visits: int) -> float:
        """
        Upper Confidence Bound applied to Trees, modified with PHI.
        Balances exploiting known good architectures vs exploring new mutations.
        """
        if self.visits == 0:
            return float('inf') # Force exploration of unvisited nodes
        
        exploitation = self.exploitation_score
        # PHI replaces the standard sqrt(2) exploration constant for fractal balance
        exploration = PHI * math.sqrt(math.log(parent_visits) / self.visits)
        return exploitation + exploration

class ScientificMCTS:
    """
    Manages the tree search of architectural hypotheses.
    """
    def __init__(self, root_hypothesis: Hypothesis):
        self.root = ExperimentNode(
            id="ROOT",
            parent_id=None,
            hypothesis=root_hypothesis,
            status="SUCCESS",
            actual_q_score=0.5 # Baseline
        )
        self.nodes: Dict[str, ExperimentNode] = {self.root.id: self.root}
        self.total_iterations = 0

    def add_hypothesis(self, parent_id: str, hypothesis: Hypothesis) -> str:
        """Branches a new hypothesis from an existing node."""
        if parent_id not in self.nodes:
            raise ValueError(f"Parent node {parent_id} not found.")
            
        node = ExperimentNode(
            id=hypothesis.id,
            parent_id=parent_id,
            hypothesis=hypothesis
        )
        self.nodes[node.id] = node
        self.nodes[parent_id].children.append(node)
        logger.info(f"[MCTS] Added hypothesis {node.id} under {parent_id}")
        return node.id

    def select_next_experiment(self) -> Optional[ExperimentNode]:
        """
        Traverses the tree using UCT to find the most promising PENDING leaf node.
        """
        current = self.root
        
        while current.children:
            unvisited = [c for c in current.children if c.visits == 0]
            if unvisited:
                return unvisited[0] # Immediately explore unvisited children
                
            # Select child with highest UCT score
            current = max(current.children, key=lambda c: c.uct_score(current.visits))
            
            # If we hit a pending node during traversal, it's our next experiment
            if current.status == "PENDING":
                return current
                
        # If the leaf is not pending (e.g. already succeeded), we need to generate new hypotheses
        # This signals the LLM to expand this node.
        if current.status == "SUCCESS":
             logger.info(f"[MCTS] Node {current.id} is a successful leaf. Needs expansion.")
             return current # Return it to signal expansion needed
             
        return None

    def backpropagate(self, node_id: str, q_score: float, status: str):
        """
        Updates the node and its ancestors with the empirical results of the experiment.
        """
        if node_id not in self.nodes:
            raise ValueError(f"Node {node_id} not found.")
            
        current: Optional[ExperimentNode] = self.nodes[node_id]
        current.status = status
        current.actual_q_score = q_score
        
        # Backpropagate the Q-score up the tree
        while current is not None:
            current.visits += 1
            current.total_q_score += q_score
            current = self.nodes.get(current.parent_id) if current.parent_id else None
            
        self.total_iterations += 1
        logger.info(f"[MCTS] Backpropagated Q={q_score:.2f} from {node_id}. Status: {status}")

    def get_best_path(self) -> List[ExperimentNode]:
        """Extracts the sequence of successful experiments with the highest Q-scores."""
        path = [self.root]
        current = self.root
        
        while current.children:
            successful_children = [c for c in current.children if c.status == "SUCCESS"]
            if not successful_children:
                break
            # Exploit only
            current = max(successful_children, key=lambda c: c.exploitation_score)
            path.append(current)
            
        return path
