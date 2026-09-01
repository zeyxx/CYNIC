#!/usr/bin/env python3
"""
CYNIC Market Simulation v4
Tests Q4 (Bayesian Truth Serum convergence) and Q5 (Collusion threshold).
"""

import random
import statistics
import math
from dataclasses import dataclass, field
from typing import List, Dict, Optional, Tuple
import json


# ============ BAYESIAN TRUTH SERUM ============

@dataclass
class Proposal:
    """A proposal with a true outcome (ground truth)."""
    proposal_id: str
    true_outcome: float  # 0-1, the actual result
    complexity: float    # 0-1, how hard to predict


@dataclass
class Prediction:
    """An actor's prediction on a proposal."""
    actor_id: str
    proposal_id:str
    predicted_outcome: float   # What they predict
    confidence: float          # 0-1, how confident
    predicted_others_avg: float  # What they think others will predict


@dataclass
class Actor:
    """Synthetic actor with BTS-aware behavior."""
    actor_id: str
    competence: float = 0.5        # 0-1, actual predictive ability
    honesty: float = 1.0           # 1.0 = honest, <1.0 = gaming
    bts_awareness: float = 0.0     # 0 = doesn't know about BTS, 1 = fully aware
    calibration: float = 0.5       # 0-1, how well calibrated
    
    predictions: List[Prediction] = field(default_factory=list)
    bts_scores: List[float] = field(default_factory=list)
    authority: float = 0.5
    
    def predict(self, proposal: Proposal, others_avg: float) -> Prediction:
        """Make a prediction, optionally using BTS strategy."""
        # Base prediction: competence determines accuracy
        noise = (1 - self.competence) * random.uniform(-0.3, 0.3)
        true_pred = proposal.true_outcome + noise
        
        if self.bts_awareness > 0.5 and self.honesty < 1.0:
            # BTS gaming: predict slightly better than average, understate others
            predicted = true_pred * 0.7 + others_avg * 0.3
            predicted_others = others_avg * 0.9  # Understate
        else:
            predicted = true_pred
            predicted_others = others_avg + random.uniform(-0.1, 0.1)
        
        predicted = max(0, min(1, predicted))
        predicted_others = max(0, min(1, predicted_others))
        
        # Confidence: well-calibrated actors have confidence ≈ accuracy
        confidence = self.calibration * (1 - abs(predicted - proposal.true_outcome))
        confidence = max(0.1, min(1.0, confidence))
        
        return Prediction(
            actor_id=self.actor_id,
            proposal_id=proposal.proposal_id,
            predicted_outcome=predicted,
            confidence=confidence,
            predicted_others_avg=predicted_others,
        )


def compute_bts_score(prediction: Prediction, actual_outcome: float, 
                      actual_avg: float) -> float:
    """
    Bayesian Truth Serum score.
    BTS = log(P(actual) / P(average)) + log(P(actual) / P_others_actual)
    
    Simplified: rewards being right AND being different from the crowd.
    """
    eps = 0.001
    
    # Information score: how much your prediction matches reality
    info_score = abs(actual_outcome - prediction.predicted_outcome)
    info_score = max(eps, 1 - info_score)  # Higher = better
    
    # Prediction of others: how well you predicted the average
    pred_others_score = max(eps, 1 - abs(actual_avg - prediction.predicted_others_avg))
    
    # BTS bonus: being right when others are wrong
    bts_bonus = math.log((info_score + eps) / (pred_others_score + eps))
    
    return bts_bonus


# ============ COLLUSION SIMULATION ============

@dataclass
class CollusionCluster:
    """A group of colluding actors."""
    cluster_id: str
    members: List[str]  # actor_ids
    target_outcome: float  # What they want the prediction to be
    coordination: float  # 0-1, how well they coordinate


def run_bts_simulation(n_actors: int = 100, n_proposals: int = 200, 
                       seed: int = 42) -> dict:
    """Test if BTS leads to calibrated predictions."""
    random.seed(seed)
    
    actors: List[Actor] = []
    
    # Diverse actor profiles
    profiles = [
        (0.9, 1.0, 0.0, 0.9, "expert_honest"),
        (0.7, 1.0, 0.0, 0.7, "good_honest"),
        (0.5, 1.0, 0.0, 0.5, "avg_honest"),
        (0.3, 1.0, 0.0, 0.3, "poor_honest"),
        (0.7, 0.7, 0.8, 0.5, "gamer_bts_aware"),
        (0.5, 0.6, 1.0, 0.4, "smart_gamer"),
    ]
    
    for i in range(n_actors):
        p = random.choice(profiles)
        actors.append(Actor(
            actor_id=f"actor_{i:04d}",
            competence=p[0], honesty=p[1], bts_awareness=p[2], calibration=p[3],
        ))
    
    proposals = []
    for i in range(n_proposals):
        proposals.append(Proposal(
            proposal_id=f"prop_{i:04d}",
            true_outcome=random.uniform(0, 1),
            complexity=random.uniform(0.1, 0.9),
        ))
    
    # Track metrics
    calibration_errors = []
    bts_bonuses = []
    
    for proposal in proposals:
        # First pass: collect predictions
        predictions = []
        for actor in actors:
            pred = actor.predict(proposal, others_avg=proposal.true_outcome + random.uniform(-0.2, 0.2))
            predictions.append(pred)
            actor.predictions.append(pred)
        
        # Compute actual average
        actual_avg = statistics.mean([p.predicted_outcome for p in predictions])
        
        # Second pass: compute BTS scores + authority update
        raw_deltas = {}
        for pred in predictions:
            bts = compute_bts_score(pred, proposal.true_outcome, actual_avg)
            actor = next(a for a in actors if a.actor_id == pred.actor_id)
            actor.bts_scores.append(bts)
            
            error = abs(pred.predicted_outcome - proposal.true_outcome)
            calibration_errors.append(error)
            
            # Base update: positive if error < 0.3, negative otherwise
            base_delta = (0.3 - error) * 0.1
            
            # Conformity penalty
            conformity = abs(pred.predicted_outcome - actual_avg)
            conformity_penalty = -conformity * 0.1
            
            # BTS bonus
            bts_bonus = bts * 0.002
            
            raw_deltas[actor.actor_id] = base_delta + conformity_penalty + bts_bonus
        
        # ZERO-SUM NORMALIZATION: total delta = 0
        total_delta = sum(raw_deltas.values())
        n = len(raw_deltas)
        normalized_deltas = {k: v - total_delta/n for k, v in raw_deltas.items()}
        
        # Apply
        for actor in actors:
            delta = normalized_deltas[actor.actor_id]
            delta = max(-0.05, min(0.05, delta))  # Clamp per-round
            actor.authority = max(0, min(1, actor.authority + delta))
        
        bts_bonuses.append(
            statistics.mean([compute_bts_score(p, proposal.true_outcome, actual_avg) for p in predictions])
        )
    
    # Analyze results
    avg_calibration_error = statistics.mean(calibration_errors)
    avg_bts_bonus = statistics.mean(bts_bonuses)
    
    # Correlation between authority and actual competence
    authority_competence_corr = compute_correlation(
        [a.authority for a in actors],
        [a.competence for a in actors]
    )
    
    # BTS-aware gamers vs honest
    bts_gamers = [a for a in actors if a.bts_awareness > 0.5 and a.honesty < 1.0]
    honest_actors = [a for a in actors if a.honesty >= 1.0]
    competent_honest = [a for a in honest_actors if a.competence >= 0.7]
    
    bts_gamer_authority = statistics.mean([a.authority for a in bts_gamers]) if bts_gamers else 0
    honest_authority = statistics.mean([a.authority for a in honest_actors]) if honest_actors else 0
    competent_honest_authority = statistics.mean([a.authority for a in competent_honest]) if competent_honest else 0
    
    # Convergence: does calibration error decrease over time?
    first_half_errors = calibration_errors[:len(calibration_errors)//2]
    second_half_errors = calibration_errors[len(calibration_errors)//2:]
    convergence = statistics.mean(first_half_errors) - statistics.mean(second_half_errors)
    
    return {
        "n_actors": n_actors,
        "n_proposals": n_proposals,
        "avg_calibration_error": avg_calibration_error,
        "avg_bts_bonus": avg_bts_bonus,
        "authority_competence_correlation": authority_competence_corr,
        "bts_gamer_avg_authority": bts_gamer_authority,
        "honest_avg_authority": honest_authority,
        "competent_honest_authority": competent_honest_authority,
        "bts_gaming_advantage": bts_gamer_authority - statistics.mean([a.competence for a in bts_gamers]) if bts_gamers else 0,
        "convergence_delta": convergence,
        "final_authority_distribution": {
            "mean": statistics.mean([a.authority for a in actors]),
            "stddev": statistics.stdev([a.authority for a in actors]) if len(actors) > 1 else 0,
            "min": min(a.authority for a in actors),
            "max": max(a.authority for a in actors),
        },
    }


def run_collusion_simulation(n_actors: int = 100, n_proposals: int = 200,
                              collusion_fraction: float = 0.2, seed: int = 42) -> dict:
    """Test collusion resistance with different cluster sizes."""
    random.seed(seed)
    
    n_colluders = int(n_actors * collusion_fraction)
    n_honest = n_actors - n_colluders
    
    actors: List[Actor] = []
    
    # Honest actors — diverse competence
    for i in range(n_honest):
        actors.append(Actor(
            actor_id=f"honest_{i:04d}",
            competence=random.uniform(0.4, 0.8),
            honesty=1.0,
            bts_awareness=random.uniform(0, 0.3),
            calibration=random.uniform(0.4, 0.8),
        ))
    
    # Colluding actors — COMPETENT (not just bad actors)
    # They coordinate but are still skilled
    for i in range(n_colluders):
        actors.append(Actor(
            actor_id=f"colluder_{i:04d}",
            competence=random.uniform(0.6, 0.9),  # Competent!
            honesty=random.uniform(0.5, 0.8),
            bts_awareness=random.uniform(0.5, 1.0),
            calibration=random.uniform(0.4, 0.7),
        ))
    
    proposals = []
    for i in range(n_proposals):
        proposals.append(Proposal(
            proposal_id=f"prop_{i:04d}",
            true_outcome=random.uniform(0, 1),
            complexity=random.uniform(0.1, 0.9),
        ))
    
    # HHI (Herfindahl-Hirschman Index) of authority
    def compute_hhi(actors: List[Actor]) -> float:
        total_auth = sum(a.authority for a in actors)
        if total_auth == 0:
            return 0
        shares = [a.authority / total_auth for a in actors]
        return sum(s**2 for s in shares)
    
    # Track metrics over time
    hhi_history = []
    colluder_advantage_history = []
    variance_history = []
    
    # Initialize in case proposals is empty
    colluder_auth = 0.0
    honest_auth = 0.0
    
    for proposal in proposals:
        predictions = []
        raw_deltas = {}
        for actor in actors:
            is_colluder = actor.actor_id.startswith("colluder")
            if is_colluder:
                # Colluders coordinate: predict toward target
                # But target is sometimes close to reality (smart collusion)
                collusion_target = proposal.true_outcome + random.uniform(-0.1, 0.1)
                pred_value = collusion_target + random.uniform(-0.05, 0.05)
            else:
                noise = (1 - actor.competence) * random.uniform(-0.3, 0.3)
                pred_value = proposal.true_outcome + noise
            
            pred_value = max(0, min(1, pred_value))
            
            pred = Prediction(
                actor_id=actor.actor_id,
                proposal_id=proposal.proposal_id,
                predicted_outcome=pred_value,
                confidence=actor.calibration,
                predicted_others_avg=statistics.mean([p.predicted_outcome for p in predictions]) if predictions else 0.5,
            )
            predictions.append(pred)
            actor.predictions.append(pred)
            
            # Update authority: ASYMMETRIC (zero-sum)
            error = abs(pred_value - proposal.true_outcome)
            base_delta = (0.3 - error) * 0.1
            
            # Conformity penalty
            actual_avg = statistics.mean([p.predicted_outcome for p in predictions])
            conformity = abs(pred_value - actual_avg)
            conformity_penalty = -conformity * 0.1
            
            raw_deltas[actor.actor_id] = base_delta + conformity_penalty
        
        # Zero-sum normalization
        total_delta = sum(raw_deltas.values())
        n = len(raw_deltas)
        for actor in actors:
            delta = raw_deltas[actor.actor_id] - total_delta/n
            delta = max(-0.05, min(0.05, delta))
            actor.authority = max(0, min(1, actor.authority + delta))
        
        # Track metrics
        hhi = compute_hhi(actors)
        hhi_history.append(hhi)
        
        colluder_auth = statistics.mean([a.authority for a in actors if a.actor_id.startswith("colluder")])
        honest_auth = statistics.mean([a.authority for a in actors if a.actor_id.startswith("honest")])
        colluder_advantage_history.append(colluder_auth - honest_auth)
        
        pred_variance = statistics.variance([p.predicted_outcome for p in predictions]) if len(predictions) > 1 else 0
        variance_history.append(pred_variance)
    
    # Analysis
    avg_hhi = statistics.mean(hhi_history)
    avg_colluder_advantage = statistics.mean(colluder_advantage_history)
    avg_variance = statistics.mean(variance_history)
    
    # Effective threshold: at what HHI does collusion become dangerous?
    # Dangerous = colluder advantage > 0.2 AND variance < 0.05
    dangerous_points = sum(1 for i in range(len(hhi_history)) 
                         if colluder_advantage_history[i] > 0.2 and variance_history[i] < 0.05)
    
    return {
        "n_actors": n_actors,
        "n_colluders": n_colluders,
        "collusion_fraction": collusion_fraction,
        "avg_hhi": avg_hhi,
        "avg_colluder_advantage": avg_colluder_advantage,
        "avg_variance": avg_variance,
        "dangerous_points": dangerous_points,
        "dangerous_fraction": dangerous_points / n_proposals,
        "final_colluder_authority": colluder_auth,
        "final_honest_authority": honest_auth,
        "hhi_threshold_estimate": avg_hhi,  # If dangerous, this HHI is too high
    }


def compute_correlation(xs: List[float], ys: List[float]) -> float:
    """Pearson correlation."""
    if len(xs) < 2:
        return 0
    mean_x = statistics.mean(xs)
    mean_y = statistics.mean(ys)
    cov = sum((x - mean_x) * (y - mean_y) for x, y in zip(xs, ys))
    std_x = statistics.stdev(xs) if len(xs) > 1 else 0.001
    std_y = statistics.stdev(ys) if len(ys) > 1 else 0.001
    return cov / (std_x * std_y * len(xs)) if std_x > 0 and std_y > 0 else 0


if __name__ == "__main__":
    print("=" * 70)
    print("CYNIC Market Simulation v4 — BTS + Collusion")
    print("=" * 70)
    
    # Q4: BTS Convergence
    print("\n--- Q4: Bayesian Truth Serum ---")
    bts_results = run_bts_simulation(n_actors=100, n_proposals=200, seed=42)
    print(f"  Actors: {bts_results['n_actors']}, Proposals: {bts_results['n_proposals']}")
    print(f"  Avg calibration error: {bts_results['avg_calibration_error']:.4f}")
    print(f"  Avg BTS bonus: {bts_results['avg_bts_bonus']:.4f}")
    print(f"  Authority-competence correlation: {bts_results['authority_competence_correlation']:.4f}")
    print(f"  BTS gamer avg authority: {bts_results['bts_gamer_avg_authority']:.4f}")
    print(f"  Honest avg authority: {bts_results['honest_avg_authority']:.4f}")
    print(f"  BTS gaming advantage: {bts_results['bts_gaming_advantage']:.4f}")
    print(f"  Convergence (error reduction): {bts_results['convergence_delta']:.4f}")
    print(f"  Authority distribution: mean={bts_results['final_authority_distribution']['mean']:.4f}, "
          f"std={bts_results['final_authority_distribution']['stddev']:.4f}")
    
    # Q5: Collusion at different fractions
    print("\n--- Q5: Collusion Threshold ---")
    for frac in [0.1, 0.2, 0.3, 0.4, 0.5]:
        coll_results = run_collusion_simulation(
            n_actors=100, n_proposals=200, collusion_fraction=frac, seed=42
        )
        print(f"\n  Collusion fraction: {frac:.0%} ({coll_results['n_colluders']} actors)")
        print(f"    Avg HHI: {coll_results['avg_hhi']:.4f}")
        print(f"    Avg colluder advantage: {coll_results['avg_colluder_advantage']:.4f}")
        print(f"    Avg prediction variance: {coll_results['avg_variance']:.4f}")
        print(f"    Dangerous points: {coll_results['dangerous_fraction']:.1%}")
        print(f"    Final colluder authority: {coll_results['final_colluder_authority']:.4f}")
        print(f"    Final honest authority: {coll_results['final_honest_authority']:.4f}")
        
        # Is this fraction dangerous?
        is_dangerous = coll_results['dangerous_fraction'] > 0.3 or coll_results['avg_colluder_advantage'] > 0.3
        print(f"    DANGEROUS: {'YES' if is_dangerous else 'NO'}")
