"""
Tier 1 EXPERIMENTAL: PoH Timing Round 2 — Adversarial Bot

Round 1 showed AUC=0.9999 but with trivial artifacts (min clipping, parameter mismatch).

Round 2 question: If a bot EXACTLY matches the human timing distribution
(same parameters, same bounds, same fast-move fraction), can we still
distinguish it using TEMPORAL STRUCTURE alone?

Hypothesis: Human timing has position-dependent correlations (autocorrelation,
genuine time pressure, recapture sequences) that a distribution-matched bot
without game awareness cannot replicate.

Falsification: If AUC < 0.618 with distribution-matched bot, then timing
distributions alone are INSUFFICIENT — need game-content signals.

Status: ACTIVE (started 2026-05-18)
"""

import json
import logging
import time
from pathlib import Path

import numpy as np
from scipy import stats
from sklearn.ensemble import RandomForestClassifier  # type: ignore[import-untyped]
from sklearn.model_selection import cross_val_score  # type: ignore[import-untyped]

# Import round 1 components
from poh_timing_experiment import (
    fetch_lichess_games,
    extract_timing_features,
)

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
log = logging.getLogger(__name__)

RESULTS_FILE = Path(__file__).parent / "poh_timing_round2_results.json"


def fit_human_distribution(human_games: list[list[float]]) -> dict[str, float]:
    """Fit the exact parameters of the human timing distribution."""
    all_times = np.array([t for g in human_games for t in g])
    all_times = all_times[all_times > 0]  # safety

    # Fit log-normal
    log_times = np.log(all_times)
    ln_mu = float(np.mean(log_times))
    ln_sigma = float(np.std(log_times))

    # Measure structural properties
    fast_frac = float(np.mean(all_times < 1.0))
    min_time = float(np.min(all_times))
    max_time = float(np.max(all_times))
    median_time = float(np.median(all_times))

    # Per-game autocorrelation distribution
    autocorrs = []
    pressure_ratios = []
    for g in human_games:
        t = np.array(g)
        if len(t) > 5:
            ac = np.corrcoef(t[:-1], t[1:])[0, 1]
            if not np.isnan(ac):
                autocorrs.append(ac)
        if len(t) > 15:
            pr = np.mean(t[-10:]) / np.mean(t[: len(t) // 2])
            if not np.isnan(pr) and np.isfinite(pr):
                pressure_ratios.append(pr)

    params = {
        "ln_mu": ln_mu,
        "ln_sigma": ln_sigma,
        "fast_frac": fast_frac,
        "min_time": min_time,
        "max_time": max_time,
        "median_time": median_time,
        "mean_autocorr": float(np.mean(autocorrs)) if autocorrs else 0.0,
        "std_autocorr": float(np.std(autocorrs)) if autocorrs else 0.1,
        "mean_pressure": float(np.mean(pressure_ratios)) if pressure_ratios else 1.0,
        "std_pressure": float(np.std(pressure_ratios)) if pressure_ratios else 0.1,
    }

    log.info("Human distribution fitted:")
    for k, v in params.items():
        log.info("  %s = %.4f", k, v)

    return params


class DistributionMatchedBot:
    """Bot that exactly matches human marginal distribution.

    Uses same log-normal params, same bounds, same fast-move fraction.
    The ONLY thing missing: position-dependent temporal correlation.
    Times are IID draws from the fitted distribution.
    """
    name = "distribution_matched"

    def __init__(self, params: dict[str, float]) -> None:
        self.params = params

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        times = rng.lognormal(
            mean=self.params["ln_mu"],
            sigma=self.params["ln_sigma"],
            size=n_moves,
        )
        times = np.clip(times, self.params["min_time"], self.params["max_time"])

        # Match fast-move fraction exactly
        target_fast = int(n_moves * self.params["fast_frac"])
        if target_fast > 0:
            fast_idx = rng.choice(n_moves, size=target_fast, replace=False)
            times[fast_idx] = rng.uniform(
                self.params["min_time"], 1.0, size=target_fast
            )

        return times.tolist()


class StructureAwareBot:
    """Bot that matches distribution AND fakes temporal structure.

    Adds synthetic autocorrelation and fake time pressure.
    This is the HARDEST adversary — matches both marginal AND some structure.
    """
    name = "structure_aware"

    def __init__(self, params: dict[str, float]) -> None:
        self.params = params

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        # Generate base with autocorrelation via AR(1) process
        target_autocorr = self.params["mean_autocorr"]
        innovation_sigma = self.params["ln_sigma"] * np.sqrt(1 - target_autocorr ** 2)

        log_times = np.zeros(n_moves)
        log_times[0] = rng.normal(self.params["ln_mu"], self.params["ln_sigma"])
        for i in range(1, n_moves):
            log_times[i] = (
                self.params["ln_mu"] * (1 - target_autocorr)
                + target_autocorr * log_times[i - 1]
                + rng.normal(0, max(innovation_sigma, 0.01))
            )

        times = np.exp(log_times)
        times = np.clip(times, self.params["min_time"], self.params["max_time"])

        # Fake time pressure: scale last moves
        if n_moves > 15:
            target_pressure = self.params["mean_pressure"]
            current_ratio = np.mean(times[-10:]) / np.mean(times[: n_moves // 2])
            if current_ratio > 0:
                scale = target_pressure / current_ratio
                times[-10:] *= scale
                times = np.clip(times, self.params["min_time"], self.params["max_time"])

        # Match fast-move fraction
        target_fast = int(n_moves * self.params["fast_frac"])
        if target_fast > 0:
            fast_idx = rng.choice(n_moves, size=min(target_fast, n_moves), replace=False)
            times[fast_idx] = rng.uniform(
                self.params["min_time"], 1.0, size=len(fast_idx)
            )

        return times.tolist()


class ReplayBot:
    """Bot that replays actual human timing sequences with perturbation.

    Picks a random human game, applies small noise. The ultimate adversary:
    if this passes, timing CANNOT distinguish human from bot.
    """
    name = "replay"

    def __init__(self, human_games: list[list[float]]) -> None:
        self.human_games = human_games

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        # Pick a random human game
        source = self.human_games[rng.integers(len(self.human_games))]

        # Resample to target length
        if len(source) >= n_moves:
            times = np.array(source[:n_moves])
        else:
            # Tile and truncate
            repeats = (n_moves // len(source)) + 1
            times = np.tile(source, repeats)[:n_moves]

        # Add small perturbation (5% gaussian noise)
        noise = rng.normal(1.0, 0.05, size=n_moves)
        times = times * noise
        times = np.clip(times, 0.01, 120.0)

        return times.tolist()


def run_round2() -> dict:
    """Round 2: adversarial bot experiment."""
    rng = np.random.default_rng(2026)
    results: dict = {
        "round": 2,
        "hypothesis": "Temporal structure (not distribution) separates human from bot",
        "falsification": "AUC < 0.618 with structure-aware bot = timing insufficient",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
    }

    # Fetch human games (reuse round 1 data)
    log.info("=== Fetching human games ===")
    human_games = fetch_lichess_games(n_games=200)
    results["human_games_count"] = len(human_games)

    # Fit human distribution
    log.info("=== Fitting human distribution ===")
    params = fit_human_distribution(human_games)
    results["fitted_params"] = {k: round(v, 4) for k, v in params.items()}

    # Create adversarial bots
    bots = [
        DistributionMatchedBot(params),
        StructureAwareBot(params),
        ReplayBot(human_games),
    ]

    # Extract human features
    human_features = [f for f in (extract_timing_features(g) for g in human_games) if f]
    feature_names = sorted(human_features[0].keys())
    results["features_used"] = feature_names
    results["per_model"] = {}

    X_human = np.array([[f[k] for k in feature_names] for f in human_features])

    for bot in bots:
        log.info("=== Testing human vs %s ===", bot.name)

        # Generate bot games with same move counts as human
        bot_games: list[list[float]] = []
        for _ in range(200):
            n = int(rng.integers(20, 60))
            bot_games.append(bot.generate_game(n, rng))

        bot_features = [f for f in (extract_timing_features(g) for g in bot_games) if f]
        X_bot = np.array([[f[k] for k in feature_names] for f in bot_features])

        X = np.nan_to_num(np.vstack([X_human, X_bot]), nan=0.0, posinf=100.0, neginf=-100.0)
        y = np.array([1] * len(X_human) + [0] * len(X_bot))

        clf = RandomForestClassifier(n_estimators=200, random_state=42, max_depth=6)
        auc_scores = cross_val_score(clf, X, y, cv=5, scoring="roc_auc")
        mean_auc = float(np.mean(auc_scores))

        clf.fit(X, y)
        importances = dict(zip(feature_names, clf.feature_importances_.tolist()))
        top_5 = sorted(importances.items(), key=lambda x: x[1], reverse=True)[:5]

        # Per-feature distribution comparison
        feature_diffs: dict[str, dict[str, float]] = {}
        for i, fname in enumerate(feature_names):
            h_vals = X_human[:, i]
            b_vals = X_bot[:, i]
            ks_s, ks_p = stats.ks_2samp(h_vals, b_vals)
            feature_diffs[fname] = {
                "human_mean": round(float(np.mean(h_vals)), 4),
                "bot_mean": round(float(np.mean(b_vals)), 4),
                "ks_stat": round(float(ks_s), 4),
                "ks_pval": float(ks_p),
            }

        # Find features where bot is indistinguishable (p > 0.05)
        matched_features = [f for f, d in feature_diffs.items() if d["ks_pval"] > 0.05]
        leaked_features = [f for f, d in feature_diffs.items() if d["ks_pval"] < 0.001]

        results["per_model"][bot.name] = {
            "auc_mean": round(mean_auc, 4),
            "auc_std": round(float(np.std(auc_scores)), 4),
            "auc_scores": [round(s, 4) for s in auc_scores.tolist()],
            "passes_threshold": mean_auc > 0.618,
            "top_features": {k: round(v, 4) for k, v in top_5},
            "matched_features": matched_features,
            "leaked_features": leaked_features,
            "feature_details": feature_diffs,
        }

        verdict = "PASS" if mean_auc > 0.618 else "FAIL"
        log.info("  AUC = %.4f +/- %.4f [%s]", mean_auc, float(np.std(auc_scores)), verdict)
        log.info("  Top: %s", ", ".join(f"{k}={v:.3f}" for k, v in top_5))
        log.info("  Matched features (bot ≈ human): %s", matched_features)
        log.info("  Leaked features (bot ≠ human): %s", leaked_features[:5])

    # Overall verdict
    replay_auc = results["per_model"].get("replay", {}).get("auc_mean", 0)
    struct_auc = results["per_model"].get("structure_aware", {}).get("auc_mean", 0)
    hardest_auc = min(replay_auc, struct_auc)

    results["verdict"] = {
        "hardest_bot": "replay" if replay_auc < struct_auc else "structure_aware",
        "hardest_auc": hardest_auc,
        "timing_sufficient": hardest_auc > 0.618,
        "interpretation": (
            f"Timing STILL sufficient (AUC={hardest_auc:.4f}) — temporal structure leaks humanity"
            if hardest_auc > 0.618
            else f"Timing INSUFFICIENT (AUC={hardest_auc:.4f}) — adversarial bots pass. Need game-content signals."
        ),
    }

    with open(RESULTS_FILE, "w") as f:
        json.dump(results, f, indent=2, default=str)
    log.info("Results saved to %s", RESULTS_FILE)

    return results


if __name__ == "__main__":
    results = run_round2()
    print("\n" + "=" * 60)
    print("POH TIMING ROUND 2 — ADVERSARIAL RESULTS")
    print("=" * 60)
    for model_name, mr in results["per_model"].items():
        auc = mr["auc_mean"]
        v = "PASS" if mr["passes_threshold"] else "FAIL"
        matched = len(mr["matched_features"])
        total = len(results["features_used"])
        print(f"  Human vs {model_name:25s}: AUC = {auc:.4f} [{v}]  ({matched}/{total} features matched)")
    print(f"\n  VERDICT: {results['verdict']['interpretation']}")
    print("=" * 60)
