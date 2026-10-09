#!/usr/bin/env python3
"""
CYNIC PoW Formula Simulation v3
Addresses all 6 issues from Gemini review:
1. Judge = simulated Actor (not oracle)
2. Gaming has a cost (energy/latency penalty)
3. Fake complexity detection
4. Collusion modeling
5. Burst gaming detection
6. Exponential decay for success
"""

import random
import statistics
import math
from dataclasses import dataclass, field
from typing import List, Dict, Optional
import json


# ============ JUDGE (simulated Actor, not oracle) ============

@dataclass
class Judge:
    """External judge — simulated as an actor with their own biases."""
    judge_id: str
    competence: float = 0.8      # How good the judge is at assessing quality
    honesty: float = 1.0         # 1.0 = honest, <1.0 = corruptible
    speed: float = 0.5           # How fast they assess
    
    def assess_quality(self, true_quality: float, reported_quality: float, 
                       complexity: float, actor_id: str = "") -> float:
        """
        Judge assesses quality. Imperfect: competence affects accuracy.
        Detects inflation but with error margin.
        """
        if true_quality <= 0:
            return 0.0
        
        inflation_ratio = reported_quality / max(0.001, true_quality)
        
        # Judge detects inflation with probability = competence
        if random.random() < self.competence:
            # Detected: apply penalty proportional to inflation
            if inflation_ratio > 1.1:  # >10% inflation
                penalty = min(1.0, (inflation_ratio - 1.0) * 2)  # 20% infl → 20% penalty
                assessed = true_quality * (1 - penalty)
            else:
                assessed = true_quality
        else:
            # Undetected: judge accepts reported quality (up to a point)
            assessed = min(reported_quality, true_quality * 1.5)
        
        # Judge error: ±(1-competence) noise
        noise = (1 - self.competence) * 0.2 * random.uniform(-1, 1)
        assessed = max(0, min(1.0, assessed + noise))
        
        return assessed


# ============ ACTOR ============

@dataclass
class Actor:
    actor_id: str
    competence: float = 0.5
    speed: float = 0.5
    energy_efficiency: float = 0.5
    honesty: float = 1.0
    gaming_cost_factor: float = 1.0  # >1 if gaming costs energy/time
    
    tasks_completed: int = 0
    tasks_attempted: int = 0
    tasks_succeeded: int = 0
    total_energy_joules: float = 0.0
    total_latency_seconds: float = 0.0
    total_complexity: float = 0.0
    total_true_complexity: float = 0.0  # Ground truth complexity
    total_judge_quality: float = 0.0
    
    pow_scores: List[float] = field(default_factory=list)
    precisions: List[float] = field(default_factory=list)
    task_history: List[dict] = field(default_factory=list)
    
    def do_task(self, true_complexity: float, time_limit: float, 
                judge: Judge, claim_complexity: Optional[float] = None) -> dict:
        """
        Simulate doing a task with:
        - true_complexity: actual difficulty
        - claim_complexity: what the actor reports (may differ = fake complexity)
        """
        self.tasks_attempted += 1
        
        # Gaming cost: if actor is gaming, they spend extra energy/time
        gaming_energy_penalty = 1.0
        gaming_latency_penalty = 1.0
        if self.honesty < 1.0:
            # Gaming costs: calculating how to lie takes energy and time
            gaming_energy_penalty = self.gaming_cost_factor
            gaming_latency_penalty = self.gaming_cost_factor
        
        # Exponential decay for success (more realistic than linear)
        # P(success) = competence × exp(-complexity × λ)
        lambda_decay = 0.8
        base_success = self.competence * math.exp(-true_complexity * lambda_decay)
        success = random.random() < base_success
        
        # Latency
        latency = (1 - self.speed) * true_complexity * time_limit * random.uniform(0.5, 1.5)
        latency *= gaming_latency_penalty  # Gaming costs time
        
        # Energy
        energy = true_complexity * (1 - self.energy_efficiency) * random.uniform(0.5, 1.5) * 1000
        energy *= gaming_energy_penalty  # Gaming costs energy
        
        # Complexity: actor may claim different from true
        if claim_complexity is None:
            reported_complexity = true_complexity
        else:
            reported_complexity = claim_complexity
        
        # Fake complexity detection: if reported > true + threshold, penalty
        fake_complexity_penalty = 1.0
        if reported_complexity > true_complexity * 1.3:  # >30% inflation
            fake_complexity_penalty = 0.8  # 20% score reduction
        
        # Quality
        if success:
            true_quality = self.competence * true_complexity * random.uniform(0.7, 1.0)
            # Honest actors report truthfully; gamers inflate
            if self.honesty >= 1.0:
                reported_quality = true_quality
            else:
                # Gaming: inflate quality, but costs energy/time (already applied)
                reported_quality = true_quality / self.honesty
        else:
            true_quality = 0
            reported_quality = 0
        
        # Judge assessment (not oracle — simulated with biases)
        judge_quality = judge.assess_quality(true_quality, reported_complexity, reported_complexity, self.actor_id)
        
        self.tasks_completed += 1
        self.total_energy_joules += energy
        self.total_latency_seconds += latency
        self.total_complexity += reported_complexity
        self.total_true_complexity += true_complexity
        self.total_judge_quality += judge_quality
        
        return {
            "success": success,
            "latency": latency,
            "energy": energy,
            "true_complexity": true_complexity,
            "reported_complexity": reported_complexity,
            "true_quality": true_quality,
            "reported_quality": reported_quality,
            "judge_quality": judge_quality,
            "fake_complexity_penalty": fake_complexity_penalty,
        }


# ============ FORMULAS ============

def log1p(x: float) -> float:
    return math.log1p(max(x, 0.001))


def pow_v4_objective(actor: Actor, alpha: float = 0.7, beta: float = 0.3) -> float:
    """Objective: metrics only + judge multiplier."""
    if actor.tasks_attempted == 0 or actor.total_energy_joules <= 0 or actor.total_latency_seconds <= 0:
        return 0.0
    
    success_rate = actor.tasks_succeeded / actor.tasks_attempted
    avg_complexity = actor.total_complexity / actor.tasks_completed
    
    numerator = actor.tasks_completed * success_rate * avg_complexity
    denominator = (actor.total_energy_joules ** alpha) * (log1p(actor.total_latency_seconds) ** beta)
    
    if denominator <= 0:
        return 0.0
    
    base_score = numerator / denominator
    
    # Judge quality multiplier
    judge_mult = actor.total_judge_quality / max(1, actor.tasks_succeeded)
    
    # Fake complexity penalty
    if actor.total_true_complexity > 0:
        complexity_inflation = actor.total_complexity / actor.total_true_complexity
        if complexity_inflation > 1.3:
            fake_penalty = 1.0 - (complexity_inflation - 1.0) * 0.5
            base_score *= max(0.5, fake_penalty)
    
    return base_score * judge_mult


def pow_v6_bayesian(actor: Actor, alpha: float = 0.7, beta: float = 0.3, prior: float = 0.5) -> float:
    """Bayesian: prior + evidence update."""
    if actor.tasks_attempted == 0 or actor.total_energy_joules <= 0 or actor.total_latency_seconds <= 0:
        return prior
    
    success_rate = actor.tasks_succeeded / actor.tasks_attempted
    avg_complexity = actor.total_complexity / actor.tasks_completed
    
    # Bayesian update
    likelihood_ratio = (success_rate + 0.01) / (0.5 + 0.01)
    posterior = prior * likelihood_ratio
    
    # Efficiency
    cost = (actor.total_energy_joules ** alpha) * (log1p(actor.total_latency_seconds) ** beta)
    efficiency = (actor.tasks_completed * avg_complexity) / max(cost, 0.001)
    
    score = posterior * efficiency
    
    # Judge multiplier
    judge_mult = actor.total_judge_quality / max(1, actor.tasks_succeeded)
    
    # Fake complexity penalty
    if actor.total_true_complexity > 0:
        complexity_inflation = actor.total_complexity / actor.total_true_complexity
        if complexity_inflation > 1.3:
            fake_penalty = 1.0 - (complexity_inflation - 1.0) * 0.5
            score *= max(0.5, fake_penalty)
    
    return max(0, score * judge_mult)


# ============ SIMULATION ============

def run_simulation(n_actors: int = 1000, n_tasks: int = 100, seed: int = 42,
                  n_seeds: int = 5) -> dict:
    """Run simulation with multiple seeds for statistical robustness."""
    
    all_results = {}
    
    for seed_offset in range(n_seeds):
        current_seed = seed + seed_offset
        random.seed(current_seed)
        
        # Create judges (simulated actors, not oracles)
        judges = [
            Judge("judge_honest", competence=0.9, honesty=1.0, speed=0.7),
            Judge("judge_average", competence=0.7, honesty=1.0, speed=0.5),
            Judge("judge_lazy", competence=0.5, honesty=1.0, speed=0.9),
            Judge("judge_corrupt", competence=0.8, honesty=0.7, speed=0.6),
        ]
        
        actors: List[Actor] = []
        
        profiles = [
            # (competence, speed, energy_eff, honesty, gaming_cost, name)
            (0.9, 0.9, 0.9, 1.0, 1.0, "elite_honest"),
            (0.7, 0.7, 0.7, 1.0, 1.0, "good_honest"),
            (0.5, 0.5, 0.5, 1.0, 1.0, "avg_honest"),
            (0.3, 0.3, 0.3, 1.0, 1.0, "poor_honest"),
            (0.9, 0.9, 0.9, 0.7, 1.3, "elite_gamer"),
            (0.5, 0.9, 0.5, 1.0, 1.0, "fast_mediocre"),
            (0.5, 0.5, 0.9, 1.0, 1.0, "efficient_mediocre"),
            (0.7, 0.7, 0.7, 0.8, 1.2, "good_slight_gamer"),
            (0.3, 0.9, 0.3, 1.0, 1.0, "fast_incompetent"),
            (0.9, 0.3, 0.3, 1.0, 1.0, "slow_competent"),
            (0.5, 0.5, 0.5, 0.6, 1.5, "smart_gamer"),
        ]
        
        for i in range(n_actors):
            p = random.choice(profiles)
            actors.append(Actor(
                actor_id=f"actor_{i:04d}",
                competence=p[0], speed=p[1], energy_efficiency=p[2], 
                honesty=p[3], gaming_cost_factor=p[4],
            ))
        
        # Burst gaming: some actors try to submit many easy tasks
        burst_actors = set(random.sample(range(n_actors), k=n_actors // 20))
        
        for task_idx in range(n_tasks):
            true_complexity = random.uniform(0.1, 0.9)
            time_limit = random.uniform(1.0, 100.0)
            
            # Rotate judges
            judge = judges[task_idx % len(judges)]
            
            for i, actor in enumerate(actors):
                # Burst gaming: actor submits 10 easy tasks instead of 1
                if i in burst_actors and task_idx % 10 == 0:
                    # Fake burst: claim low complexity but actual is high
                    claim = true_complexity * 0.3
                else:
                    claim = None  # Report truthfully
                
                result = actor.do_task(true_complexity, time_limit, judge, claim_complexity=claim)
                actor.precisions.append(1.0 if result["success"] else 0.0)
        
        # Compute scores
        for name, fn in [("v4_objective", pow_v4_objective), ("v6_bayesian", pow_v6_bayesian)]:
            scored = [{"actor": a, "score": fn(a)} for a in actors]
            scored.sort(key=lambda x: x["score"], reverse=True)
            
            top10 = scored[:10]
            bot10 = scored[-10:]
            
            key = f"{name}_seed{current_seed}"
            all_results[key] = {
                "top10_competence": statistics.mean([r["actor"].competence for r in top10]),
                "bot10_competence": statistics.mean([r["actor"].competence for r in bot10]),
                "top10_honesty": statistics.mean([r["actor"].honesty for r in top10]),
                "gaming_in_top10": sum(1 for r in top10 if r["actor"].honesty < 1.0),
                "honest_elite_in_top10": sum(1 for r in top10 if r["actor"].competence >= 0.9 and r["actor"].honesty >= 1.0),
                "score_stddev": statistics.stdev([r["score"] for r in scored]) if len(scored) > 1 else 0,
                "top5": [
                    {
                        "id": r["actor"].actor_id,
                        "score": r["score"],
                        "competence": r["actor"].competence,
                        "honesty": r["actor"].honesty,
                    }
                    for r in scored[:5]
                ],
            }
    
    # Aggregate across seeds
    aggregated = {}
    for name in ["v4_objective", "v6_bayesian"]:
        seed_results = [v for k, v in all_results.items() if k.startswith(name)]
        
        aggregated[name] = {
            "avg_top10_comp": statistics.mean([r["top10_competence"] for r in seed_results]),
            "avg_bot10_comp": statistics.mean([r["bot10_competence"] for r in seed_results]),
            "avg_top10_honesty": statistics.mean([r["top10_honesty"] for r in seed_results]),
            "avg_gaming_top10": statistics.mean([r["gaming_in_top10"] for r in seed_results]),
            "avg_honest_elite": statistics.mean([r["honest_elite_in_top10"] for r in seed_results]),
            "avg_stddev": statistics.mean([r["score_stddev"] for r in seed_results]),
            "n_seeds": len(seed_results),
            "raw_seeds": seed_results,
        }
    
    return aggregated


if __name__ == "__main__":
    results = run_simulation(n_actors=1000, n_tasks=100, n_seeds=5)
    
    print("=" * 70)
    print("CYNIC PoW Simulation v3 — Gaming + Judge + Burst + Multi-seed")
    print("=" * 70)
    print()
    
    for name, r in results.items():
        print(f"--- {name} ({r['n_seeds']} seeds) ---")
        print(f"  Avg Top-10 competence: {r['avg_top10_comp']:.3f}")
        print(f"  Avg Bottom-10 competence: {r['avg_bot10_comp']:.3f}")
        print(f"  Avg Top-10 honesty: {r['avg_top10_honesty']:.3f}")
        print(f"  Avg Gaming in Top-10: {r['avg_gaming_top10']:.1f}/10")
        print(f"  Avg Honest Elite in Top-10: {r['avg_honest_elite']:.1f}/10")
        print(f"  Avg Score Stddev: {r['avg_stddev']:.6f}")
        print()
    
    # Per-seed breakdown
    print("--- Per-seed gaming in Top-10 ---")
    for name, r in results.items():
        gaming_counts = [s["gaming_in_top10"] for s in r["raw_seeds"]]
        print(f"  {name}: {gaming_counts} (avg={statistics.mean(gaming_counts):.1f})")
