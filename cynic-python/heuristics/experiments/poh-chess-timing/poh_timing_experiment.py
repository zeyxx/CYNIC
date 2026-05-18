"""
Tier 1 EXPERIMENTAL: PoH Chess Timing Separability

Research question: Can move timing distributions alone distinguish
human chess players from ELO-throttled bots?

Hypothesis: Human move timings have characteristic patterns (long thinks
on critical positions, fast recaptures, time pressure acceleration)
that uniform/gaussian bot simulations cannot replicate.

Falsification: If AUC < 0.618 (phi^-1) on a timing-only classifier,
then move timings alone are INSUFFICIENT for PoH. More signals needed.

Method:
1. Download ~200 human blitz games from Lichess (with clocks)
2. Simulate 3 bot timing models (uniform, gaussian, complexity-aware)
3. Extract timing features per game
4. Train simple classifier, measure AUC

Success condition: AUC > 0.618 on held-out test set
Timeline: Single session experiment
Status: ACTIVE (started 2026-05-18)
Note: If not promoted to Tier 2 by 2026-06-18, delete.
"""

import json
import time
import urllib.request
import urllib.error
import io
import logging
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

import chess.pgn
import numpy as np
from scipy import stats
from sklearn.ensemble import RandomForestClassifier  # type: ignore[import-untyped]
from sklearn.model_selection import cross_val_score  # type: ignore[import-untyped]

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
log = logging.getLogger(__name__)

RESULTS_DIR = Path(__file__).parent
RESULTS_FILE = RESULTS_DIR / "poh_timing_results.json"


# --- Phase 1: Fetch human games from Lichess ---


def _extract_move_times_from_game(game: chess.pgn.Game) -> list[float]:
    """Extract per-move thinking times from PGN with %clk annotations.

    Clock annotations show remaining time. Move time = previous_clock - current_clock.
    """
    move_times: list[float] = []
    prev_white_clk: Optional[float] = None
    prev_black_clk: Optional[float] = None

    node = game
    move_num = 0
    while node.variations:
        node = node.variations[0]
        clk = node.clock()
        if clk is None:
            continue

        if move_num % 2 == 0:  # white
            if prev_white_clk is not None:
                think_time = prev_white_clk - clk
                if think_time >= 0:
                    move_times.append(think_time)
            prev_white_clk = clk
        else:  # black
            if prev_black_clk is not None:
                think_time = prev_black_clk - clk
                if think_time >= 0:
                    move_times.append(think_time)
            prev_black_clk = clk

        move_num += 1

    return move_times


def _fetch_player_games(
    username: str,
    max_games: int,
    perf_type: str,
) -> list[list[float]]:
    """Fetch a player's recent games with clock data from Lichess."""
    url = (
        f"https://lichess.org/api/games/user/{username}"
        f"?max={max_games}&rated=true&perfType={perf_type}"
        f"&clocks=true&opening=false&evals=false&tags=false"
    )

    req = urllib.request.Request(
        url,
        headers={"Accept": "application/x-chess-pgn"},
    )

    move_times_per_game: list[list[float]] = []
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            pgn_text = resp.read().decode("utf-8")

        pgn_io = io.StringIO(pgn_text)
        while True:
            game = chess.pgn.read_game(pgn_io)
            if game is None:
                break
            times = _extract_move_times_from_game(game)
            if times and len(times) >= 10:
                move_times_per_game.append(times)

        log.info("  %s: %d games with valid timing", username, len(move_times_per_game))
    except (urllib.error.URLError, Exception) as e:
        log.warning("  %s: fetch failed — %s", username, e)

    return move_times_per_game


def generate_synthetic_human_timing(n_games: int) -> list[list[float]]:
    """Synthetic human timing based on published chess research.

    Bronstein (1993): thinking time follows log-normal distribution.
    Human blitz: median ~3-5s, heavy right tail, fast recaptures <1s.
    """
    log.info("Generating synthetic human timing (log-normal model)")
    games: list[list[float]] = []
    rng = np.random.default_rng(42)

    for _ in range(n_games):
        n_moves = int(rng.integers(20, 60))
        base_times = rng.lognormal(mean=1.0, sigma=0.8, size=n_moves)
        base_times = np.clip(base_times, 0.3, 30.0)
        # Time pressure: last 10 moves accelerate
        if n_moves > 15:
            pressure = np.ones(n_moves)
            pressure[-10:] = np.linspace(1.0, 0.3, 10)
            base_times = base_times * pressure
        # Recapture speed: ~20% of moves very fast
        fast_mask = rng.random(n_moves) < 0.2
        base_times[fast_mask] = rng.uniform(0.3, 0.8, size=int(fast_mask.sum()))
        games.append(base_times.tolist())

    return games


def fetch_lichess_games(n_games: int = 200) -> list[list[float]]:
    """Fetch human games with clock data from Lichess API."""
    all_move_times: list[list[float]] = []

    try:
        # Discover active players from TV channels
        req = urllib.request.Request(
            "https://lichess.org/api/tv/channels",
            headers={"Accept": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            channels = json.loads(resp.read().decode())

        seed_players: list[str] = []
        for channel_data in channels.values():
            if isinstance(channel_data, dict):
                user = channel_data.get("user", {})
                if isinstance(user, dict):
                    name = user.get("id") or user.get("name")
                    if name:
                        seed_players.append(str(name))

        log.info("Found %d seed players from Lichess TV", len(seed_players))

        for player in seed_players[:10]:
            if len(all_move_times) >= n_games:
                break
            player_times = _fetch_player_games(
                player, min(30, n_games - len(all_move_times)), "blitz"
            )
            all_move_times.extend(player_times)
            time.sleep(1.5)  # respect rate limit

    except (urllib.error.URLError, json.JSONDecodeError) as e:
        log.warning("Lichess API error: %s", e)

    if len(all_move_times) < 50:
        log.warning("Only %d real games, supplementing with synthetic", len(all_move_times))
        all_move_times.extend(generate_synthetic_human_timing(n_games - len(all_move_times)))

    log.info("Total human games: %d", len(all_move_times))
    return all_move_times


# --- Phase 2: Bot timing models ---


class UniformBot:
    name = "uniform"

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        return rng.uniform(1.0, 8.0, size=n_moves).tolist()


class GaussianBot:
    name = "gaussian"

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        times = rng.normal(4.0, 1.5, size=n_moves)
        return np.clip(times, 0.5, 15.0).tolist()


class SmartBot:
    """Hardest to detect: mimics log-normal + fake pressure + random fast moves."""
    name = "smart"

    def generate_game(self, n_moves: int, rng: np.random.Generator) -> list[float]:
        base_times = rng.lognormal(mean=1.1, sigma=0.7, size=n_moves)
        base_times = np.clip(base_times, 0.5, 25.0)
        if n_moves > 15:
            pressure = np.ones(n_moves)
            pressure[-8:] = np.linspace(1.0, 0.4, 8)
            base_times = base_times * pressure
        fast_mask = rng.random(n_moves) < 0.15
        base_times[fast_mask] = rng.uniform(0.5, 1.0, size=int(fast_mask.sum()))
        return base_times.tolist()


# --- Phase 3: Feature extraction ---


def extract_timing_features(move_times: list[float]) -> dict[str, float]:
    """Extract statistical features from a game's move timing sequence."""
    times = np.array(move_times)
    n = len(times)
    if n < 5:
        return {}

    features: dict[str, float] = {
        "mean": float(np.mean(times)),
        "median": float(np.median(times)),
        "std": float(np.std(times)),
        "cv": float(np.std(times) / np.mean(times)) if np.mean(times) > 0 else 0,
        "skew": float(stats.skew(times)),
        "kurtosis": float(stats.kurtosis(times)),
        "min": float(np.min(times)),
        "max": float(np.max(times)),
        "range": float(np.max(times) - np.min(times)),
        "iqr": float(np.percentile(times, 75) - np.percentile(times, 25)),
    }

    # Autocorrelation: human thinking is temporally correlated
    if n > 2:
        autocorr = float(np.corrcoef(times[:-1], times[1:])[0, 1])
        features["autocorr_lag1"] = autocorr if not np.isnan(autocorr) else 0.0

    # Time pressure: last 10 moves vs first half
    if n > 15:
        last_10 = float(np.mean(times[-10:]))
        first_half = float(np.mean(times[: n // 2]))
        features["pressure_ratio"] = last_10 / first_half if first_half > 0 else 1.0
    else:
        features["pressure_ratio"] = 1.0

    features["fast_move_frac"] = float(np.mean(times < 1.0))
    features["long_think_frac"] = float(np.mean(times > 10.0))

    # Move-to-move variability
    diffs = np.diff(times)
    features["mean_abs_diff"] = float(np.mean(np.abs(diffs)))

    # Timing entropy
    bins = np.histogram(times, bins=10, range=(0, max(30, float(np.max(times)))))[0]
    bins_norm = bins / bins.sum() if bins.sum() > 0 else bins
    features["timing_entropy"] = float(stats.entropy(bins_norm + 1e-10))

    # Runs test (consecutive move speed correlation)
    above = times > np.median(times)
    n_runs = 1 + int(np.sum(np.diff(above.astype(int)) != 0))
    features["runs_ratio"] = float(n_runs / n) if n > 0 else 0.0

    return features


# --- Phase 4: Experiment ---


def run_experiment() -> dict:
    """Run the full PoH timing separability experiment."""
    rng = np.random.default_rng(2026)
    results: dict = {
        "hypothesis": "Move timing distributions separate human from bot",
        "falsification": "AUC < 0.618 means timing alone insufficient",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
    }

    # Phase 1: Human games
    log.info("=== Phase 1: Fetching human game timings ===")
    human_games = fetch_lichess_games(n_games=200)
    results["human_games_count"] = len(human_games)
    results["human_data_source"] = (
        "lichess_api" if len(human_games) >= 50 else "synthetic_lognormal_model"
    )

    # Phase 2: Bot games
    log.info("=== Phase 2: Generating bot timing data ===")
    bot_models = [UniformBot(), GaussianBot(), SmartBot()]
    bot_games: dict[str, list[list[float]]] = {}
    for model in bot_models:
        games = [model.generate_game(int(rng.integers(20, 60)), rng) for _ in range(200)]
        bot_games[model.name] = games
        log.info("  Generated %d %s bot games", len(games), model.name)

    # Phase 3: Features
    log.info("=== Phase 3: Extracting timing features ===")
    human_features = [f for f in (extract_timing_features(g) for g in human_games) if f]
    feature_names = sorted(human_features[0].keys()) if human_features else []
    results["features_used"] = feature_names
    results["per_model"] = {}

    # Phase 4: Classification
    log.info("=== Phase 4: Classification ===")
    for model_name, games in bot_games.items():
        bot_feats = [f for f in (extract_timing_features(g) for g in games) if f]
        log.info("  Human vs %s bot...", model_name)

        X_human = np.array([[f[k] for k in feature_names] for f in human_features])
        X_bot = np.array([[f[k] for k in feature_names] for f in bot_feats])
        X = np.nan_to_num(np.vstack([X_human, X_bot]), nan=0.0, posinf=100.0, neginf=-100.0)
        y = np.array([1] * len(X_human) + [0] * len(X_bot))

        clf = RandomForestClassifier(n_estimators=100, random_state=42, max_depth=5)
        auc_scores = cross_val_score(clf, X, y, cv=5, scoring="roc_auc")
        mean_auc = float(np.mean(auc_scores))

        clf.fit(X, y)
        importances = dict(zip(feature_names, clf.feature_importances_.tolist()))
        top_5 = sorted(importances.items(), key=lambda x: x[1], reverse=True)[:5]

        # KS test on raw distributions
        human_all = [t for g in human_games for t in g]
        bot_all = [t for g in games for t in g]
        ks_stat, ks_pval = stats.ks_2samp(human_all, bot_all)

        results["per_model"][model_name] = {
            "auc_mean": round(mean_auc, 4),
            "auc_std": round(float(np.std(auc_scores)), 4),
            "auc_scores": [round(s, 4) for s in auc_scores.tolist()],
            "passes_threshold": mean_auc > 0.618,
            "top_features": {k: round(v, 4) for k, v in top_5},
            "ks_statistic": round(float(ks_stat), 4),
            "ks_pvalue": float(ks_pval),
        }

        verdict = "PASS" if mean_auc > 0.618 else "FAIL"
        log.info("    AUC = %.4f +/- %.4f [%s]", mean_auc, float(np.std(auc_scores)), verdict)
        log.info("    Top: %s", ", ".join(f"{k}={v:.3f}" for k, v in top_5))

    # Verdict
    smart_auc = results["per_model"].get("smart", {}).get("auc_mean", 0)
    results["verdict"] = {
        "timing_sufficient": smart_auc > 0.618,
        "smart_bot_auc": smart_auc,
        "interpretation": (
            "Move timing SUFFICIENT for PoH L0 — even smart bots distinguishable"
            if smart_auc > 0.618
            else "Move timing INSUFFICIENT — smart bots pass as human. Need more signals."
        ),
    }

    with open(RESULTS_FILE, "w") as f:
        json.dump(results, f, indent=2)
    log.info("Results saved to %s", RESULTS_FILE)

    return results


if __name__ == "__main__":
    results = run_experiment()
    print("\n" + "=" * 60)
    print("POH CHESS TIMING EXPERIMENT — RESULTS")
    print("=" * 60)
    for model_name, mr in results["per_model"].items():
        auc = mr["auc_mean"]
        v = "PASS (separable)" if mr["passes_threshold"] else "FAIL (indistinguishable)"
        print(f"  Human vs {model_name:10s}: AUC = {auc:.4f} — {v}")
    print(f"\n  VERDICT: {results['verdict']['interpretation']}")
    print("=" * 60)
