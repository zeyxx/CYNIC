"""
Tier 1 EXPERIMENTAL: PoH Timing Round 3 — Remove All Artifacts

Round 2 showed AUC=0.999 but dominated by fast_move_frac and min artifacts.
Lichess clocks are integer seconds — human min=1.0s, bot min<1.0s.
This is API resolution, not a PoH signal.

Round 3: Force both human and bot data to integer seconds (floor).
Remove fast_move_frac and min features. Test ONLY structural features.

Question: With matching resolution and distribution, do STRUCTURAL
features (autocorrelation, pressure, runs, entropy) still separate?

Falsification: AUC < 0.618 = structure alone insufficient.

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

from poh_timing_experiment import fetch_lichess_games

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
log = logging.getLogger(__name__)

RESULTS_FILE = Path(__file__).parent / "poh_timing_round3_results.json"


def quantize_to_integer_seconds(games: list[list[float]]) -> list[list[float]]:
    """Force all times to integer seconds (like Lichess resolution)."""
    return [
        [max(1.0, float(int(t))) for t in game]
        for game in games
    ]


def extract_structural_features(move_times: list[float]) -> dict[str, float]:
    """Extract ONLY structural features — no distributional artifacts.

    Removes: min, max, range (resolution-dependent)
    Removes: fast_move_frac (Lichess integer artifact)
    Keeps: temporal structure that requires game awareness to fake.
    """
    times = np.array(move_times, dtype=float)
    n = len(times)
    if n < 10:
        return {}

    features: dict[str, float] = {}

    # --- Distributional (matched by bots, kept as baseline) ---
    features["mean"] = float(np.mean(times))
    features["median"] = float(np.median(times))
    features["std"] = float(np.std(times))
    features["cv"] = float(np.std(times) / np.mean(times)) if np.mean(times) > 0 else 0
    features["skew"] = float(stats.skew(times))
    features["kurtosis"] = float(stats.kurtosis(times))
    features["iqr"] = float(np.percentile(times, 75) - np.percentile(times, 25))

    # --- STRUCTURAL (the real test) ---

    # Autocorrelation at lags 1-3: human thinks correlate across moves
    for lag in [1, 2, 3]:
        if n > lag + 1:
            ac = float(np.corrcoef(times[:-lag], times[lag:])[0, 1])
            features[f"autocorr_lag{lag}"] = ac if not np.isnan(ac) else 0.0

    # Partial autocorrelation: lag-2 after removing lag-1 effect
    if n > 4:
        residual = times[1:-1] - features.get("autocorr_lag1", 0) * times[:-2]
        pac2 = float(np.corrcoef(residual, times[2:])[0, 1])
        features["partial_autocorr_2"] = pac2 if not np.isnan(pac2) else 0.0

    # Time pressure: acceleration in last quarter vs first half
    q1 = times[: n // 4]
    q4 = times[-n // 4:]
    first_half = times[: n // 2]
    features["pressure_q4_q1"] = float(np.mean(q4) / np.mean(q1)) if np.mean(q1) > 0 else 1.0
    features["pressure_q4_h1"] = float(np.mean(q4) / np.mean(first_half)) if np.mean(first_half) > 0 else 1.0

    # Acceleration: slope of moving average
    window = min(5, n // 4)
    if window > 1:
        ma = np.convolve(times, np.ones(window) / window, mode="valid")
        if len(ma) > 1:
            slope = np.polyfit(range(len(ma)), ma, 1)[0]
            features["trend_slope"] = float(slope)

    # Runs test: measures randomness of above/below median sequence
    median_t = np.median(times)
    above = times > median_t
    n_runs = 1 + int(np.sum(np.diff(above.astype(int)) != 0))
    features["runs_ratio"] = float(n_runs / n)

    # Expected runs for random sequence
    n_above = int(np.sum(above))
    n_below = n - n_above
    if n_above > 0 and n_below > 0:
        expected_runs = 1 + 2 * n_above * n_below / n
        features["runs_z"] = float((n_runs - expected_runs) / max(np.sqrt(expected_runs), 1))
    else:
        features["runs_z"] = 0.0

    # Timing entropy (10 bins)
    bins = np.histogram(times, bins=10, range=(1, max(30, float(np.max(times)))))[0]
    bins_norm = bins / bins.sum() if bins.sum() > 0 else bins
    features["timing_entropy"] = float(stats.entropy(bins_norm + 1e-10))

    # Move-to-move transitions: how "jumpy" is the timing?
    diffs = np.diff(times)
    features["mean_abs_diff"] = float(np.mean(np.abs(diffs)))
    features["diff_std"] = float(np.std(diffs))

    # Diff autocorrelation: are acceleration patterns structured?
    if len(diffs) > 2:
        diff_ac = float(np.corrcoef(diffs[:-1], diffs[1:])[0, 1])
        features["diff_autocorr"] = diff_ac if not np.isnan(diff_ac) else 0.0

    # Long think clustering: do long thinks come in bursts?
    long_mask = times > np.percentile(times, 75)
    if np.sum(long_mask) > 1:
        long_positions = np.where(long_mask)[0]
        long_gaps = np.diff(long_positions)
        features["long_think_clustering"] = float(np.std(long_gaps) / np.mean(long_gaps)) if np.mean(long_gaps) > 0 else 0.0
    else:
        features["long_think_clustering"] = 0.0

    return features


def fit_human_params(human_games: list[list[float]]) -> dict[str, float]:
    """Fit human timing parameters for bot generation."""
    all_times = np.array([t for g in human_games for t in g])
    log_times = np.log(all_times[all_times > 0])

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

    return {
        "ln_mu": float(np.mean(log_times)),
        "ln_sigma": float(np.std(log_times)),
        "mean_autocorr": float(np.mean(autocorrs)) if autocorrs else 0.15,
        "mean_pressure": float(np.mean(pressure_ratios)) if pressure_ratios else 1.0,
    }


class PerfectBot:
    """Bot matching distribution + integer quantization + AR(1) structure +
    pressure + long-think clustering. Maximum adversarial effort."""
    name = "perfect_mimic"

    def __init__(self, params: dict[str, float]) -> None:
        self.p = params

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        # AR(1) in log-space with fitted autocorrelation
        ac = self.p["mean_autocorr"]
        inn_sigma = self.p["ln_sigma"] * np.sqrt(max(1 - ac ** 2, 0.01))

        log_t = np.zeros(n_moves)
        log_t[0] = rng.normal(self.p["ln_mu"], self.p["ln_sigma"])
        for i in range(1, n_moves):
            log_t[i] = (
                self.p["ln_mu"] * (1 - ac)
                + ac * log_t[i - 1]
                + rng.normal(0, inn_sigma)
            )

        times = np.exp(log_t)

        # Pressure: scale last quarter
        if n_moves > 15:
            target = self.p["mean_pressure"]
            current = np.mean(times[-n_moves // 4:]) / np.mean(times[: n_moves // 2])
            if current > 0:
                times[-n_moves // 4:] *= target / current

        # Quantize to integer seconds (like Lichess)
        times = np.maximum(1.0, np.floor(times))

        return times.tolist()


class IIDBot:
    """Baseline: IID samples from exact human distribution, quantized."""
    name = "iid_quantized"

    def __init__(self, params: dict[str, float]) -> None:
        self.p = params

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        times = rng.lognormal(self.p["ln_mu"], self.p["ln_sigma"], size=n_moves)
        times = np.maximum(1.0, np.floor(times))
        return times.tolist()


class ShuffleBot:
    """Takes a real human game and shuffles the move order.
    Preserves exact marginal distribution, destroys temporal structure."""
    name = "shuffle"

    def __init__(self, human_games: list[list[float]]) -> None:
        self.games = human_games

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        source = list(self.games[rng.integers(len(self.games))])
        rng.shuffle(source)  # type: ignore[arg-type]
        if len(source) >= n_moves:
            return source[:n_moves]
        return (source * ((n_moves // len(source)) + 1))[:n_moves]


def run_round3() -> dict:
    """Round 3: artifact-free structural test."""
    rng = np.random.default_rng(2026)
    results: dict = {
        "round": 3,
        "hypothesis": "Temporal structure separates human from distribution-matched bot",
        "falsification": "AUC < 0.618 = need game-content signals",
        "artifact_controls": [
            "All times quantized to integer seconds",
            "No min/max/range/fast_move_frac features",
            "Bot uses exact fitted human parameters",
        ],
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
    }

    # Fetch and quantize human games
    log.info("=== Fetching + quantizing human games ===")
    raw_games = fetch_lichess_games(n_games=200)
    human_games = quantize_to_integer_seconds(raw_games)
    results["human_games_count"] = len(human_games)

    # Fit parameters on quantized data
    params = fit_human_params(human_games)
    results["fitted_params"] = {k: round(v, 4) for k, v in params.items()}
    log.info("Fitted params: %s", results["fitted_params"])

    # Bots
    bots = [
        IIDBot(params),
        ShuffleBot(human_games),
        PerfectBot(params),
    ]

    # Features
    human_features = [f for f in (extract_structural_features(g) for g in human_games) if f]
    feature_names = sorted(human_features[0].keys())
    results["features_used"] = feature_names
    results["n_features"] = len(feature_names)
    results["per_model"] = {}

    X_human = np.array([[f.get(k, 0.0) for k in feature_names] for f in human_features])

    for bot in bots:
        log.info("=== Human vs %s ===", bot.name)

        bot_games = [bot.generate_game(int(rng.integers(20, 60)), rng) for _ in range(200)]
        bot_features = [f for f in (extract_structural_features(g) for g in bot_games) if f]
        X_bot = np.array([[f.get(k, 0.0) for k in feature_names] for f in bot_features])

        X = np.nan_to_num(np.vstack([X_human, X_bot]), nan=0.0, posinf=100.0, neginf=-100.0)
        y = np.array([1] * len(X_human) + [0] * len(X_bot))

        clf = RandomForestClassifier(n_estimators=200, random_state=42, max_depth=6)
        auc_scores = cross_val_score(clf, X, y, cv=5, scoring="roc_auc")
        mean_auc = float(np.mean(auc_scores))

        clf.fit(X, y)
        importances = dict(zip(feature_names, clf.feature_importances_.tolist()))
        top_5 = sorted(importances.items(), key=lambda x: x[1], reverse=True)[:5]

        results["per_model"][bot.name] = {
            "auc_mean": round(mean_auc, 4),
            "auc_std": round(float(np.std(auc_scores)), 4),
            "auc_folds": [round(s, 4) for s in auc_scores.tolist()],
            "passes": mean_auc > 0.618,
            "top_features": {k: round(v, 4) for k, v in top_5},
        }

        v = "PASS" if mean_auc > 0.618 else "FAIL"
        log.info("  AUC = %.4f +/- %.4f [%s]", mean_auc, float(np.std(auc_scores)), v)
        log.info("  Top: %s", ", ".join(f"{k}={v:.3f}" for k, v in top_5))

    # Verdict
    perfect_auc = results["per_model"]["perfect_mimic"]["auc_mean"]
    shuffle_auc = results["per_model"]["shuffle"]["auc_mean"]
    hardest_auc = min(perfect_auc, shuffle_auc)
    hardest_name = "perfect_mimic" if perfect_auc <= shuffle_auc else "shuffle"

    results["verdict"] = {
        "hardest_bot": hardest_name,
        "hardest_auc": hardest_auc,
        "timing_sufficient": hardest_auc > 0.618,
        "shuffle_auc": shuffle_auc,
        "perfect_mimic_auc": perfect_auc,
        "interpretation": (
            f"Structure SUFFICIENT (AUC={hardest_auc:.4f}) — temporal patterns leak humanity even with matched distribution"
            if hardest_auc > 0.618
            else f"Structure INSUFFICIENT (AUC={hardest_auc:.4f}) — adversarial bot with matched structure passes as human"
        ),
        "key_insight": (
            f"Shuffle bot AUC={shuffle_auc:.4f}: {'temporal order matters' if shuffle_auc > 0.618 else 'temporal order does NOT help'}. "
            f"Perfect mimic AUC={perfect_auc:.4f}: {'AR(1) structure still distinguishable' if perfect_auc > 0.618 else 'AR(1) structure sufficient to fool classifier'}"
        ),
    }

    with open(RESULTS_FILE, "w") as f:
        json.dump(results, f, indent=2, default=str)
    log.info("Saved to %s", RESULTS_FILE)

    return results


if __name__ == "__main__":
    results = run_round3()
    print("\n" + "=" * 60)
    print("POH TIMING ROUND 3 — ARTIFACT-FREE STRUCTURAL TEST")
    print("=" * 60)
    for name, mr in results["per_model"].items():
        v = "PASS" if mr["passes"] else "FAIL"
        print(f"  Human vs {name:20s}: AUC = {mr['auc_mean']:.4f} [{v}]")
        top = mr["top_features"]
        print(f"    Top: {', '.join(f'{k}={v:.3f}' for k, v in list(top.items())[:3])}")
    print(f"\n  {results['verdict']['interpretation']}")
    print(f"  {results['verdict']['key_insight']}")
    print("=" * 60)
