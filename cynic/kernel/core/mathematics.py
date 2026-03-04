"""
CYNIC Core Mathematics - Foundational ML Algorithms.
Based on:
- Sutton & Barto (Reinforcement Learning): UCB1, Q-Learning updates.
- Murphy (Probabilistic ML): Bayesian updating.
- Mohri (Foundations of ML): Empirical Risk.
"""
import math
from typing import List

class RLMathematics:
    @staticmethod
    def ucb1_score(average_reward: float, total_system_trials: int, model_trials: int, exploration_constant: float = 1.414) -> float:
        """
        Upper Confidence Bound (UCB1) - Sutton & Barto, Chapter 2.
        Balances exploiting the known best model with exploring untested models.
        """
        if model_trials == 0:
            return float('inf') # Force exploration of untested models
        
        # exploration term: c * sqrt(ln(t) / N_a)
        exploration_term = exploration_constant * math.sqrt(math.log(total_system_trials) / model_trials)
        return average_reward + exploration_term

class ProbabilisticMathematics:
    @staticmethod
    def bayesian_update(prior_success_prob: float, prior_weight: int, successes: int, total_trials: int) -> float:
        """
        Bayesian Posterior updating (Beta-Binomial conjugate).
        Updates the belief in a model's success probability based on new evidence.
        """
        alpha_prior = prior_success_prob * prior_weight
        beta_prior = prior_weight - alpha_prior
        
        alpha_post = alpha_prior + successes
        beta_post = beta_prior + (total_trials - successes)
        
        return alpha_post / (alpha_post + beta_post)

class MultiDimensionalMathematics:
    @staticmethod
    def l2_norm(vector: List[float]) -> float:
        """Calculates the Euclidean norm of a performance vector."""
        if not vector: return 0.0
        sum_sq = sum(x*x for x in vector)
        return math.sqrt(sum_sq) / math.sqrt(len(vector)) # Normalized to 0-1
