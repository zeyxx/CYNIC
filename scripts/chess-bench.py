#!/usr/bin/env python3
"""Chess Benchmark — Progressive A/B stress test for CYNIC crystal loop.

Runs chess stimuli through POST /judge with crystals=true and crystals=false,
comparing Q-scores to measure crystal injection delta.

Usage: python3 scripts/chess-bench.py [wave]
  wave 1 = 10 stimuli (default)
  wave 2 = 20 stimuli
  wave 3 = 30 stimuli
"""

import json, os, sys, time, urllib.request, urllib.error

ADDR = os.environ.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
API_KEY = os.environ.get("CYNIC_API_KEY", "")
BASE = f"http://{ADDR}"
TIMEOUT = 120

# ── Stimulus corpus ──────────────────────────────────────────
# (tier, content, context)

WAVE1 = [
    ("HOWL", "1. e4 c5 — The Sicilian Defense. Black fights for the center asymmetrically.",
     "Most popular response to 1.e4 at grandmaster level. Deep theoretical lines, rich middlegame play."),
    ("GROWL", "1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# — Scholar's Mate.",
     "Beginner trap that fails against prepared opponents. Premature queen development."),
    ("BARK", "1. f3 e5 2. g4 Qh4# — Fool's Mate. Worst possible opening.",
     "No grandmaster would ever play this. Two moves to checkmate."),
    ("HOWL", "1. d4 Nf6 2. c4 g6 3. Nc3 Bg7 4. e4 d6 5. Nf3 O-O 6. Be2 e5 — King's Indian Defense, Classical.",
     "One of the most theoretically rich openings. Black allows White a broad center then counterattacks. Kasparov's weapon of choice."),
    ("WAG", "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 — Ruy Lopez, Closed.",
     "The most classical opening in chess history. Sound, strategic, balanced."),
    ("WAG", "1. d4 d5 2. c4 e6 3. Nc3 Nf6 4. Bg5 Be7 5. e3 O-O — Queen's Gambit Declined.",
     "Solid defensive system. Black develops naturally and maintains a strong pawn structure."),
    ("GROWL", "1. e4 e5 2. Bc4 Nc6 3. Qf3 — Wayward Queen Attack.",
     "Premature queen sortie targeting f7. Easily refuted by 3...Nf6. Popular among beginners."),
    ("BARK", "1. e4 e5 2. Ke2?! — The Bongcloud Attack.",
     "King moves on move 2 abandoning castling rights. A joke opening violating every classical principle."),
    ("HOWL", "1. e4 c6 2. d4 d5 3. Nc3 dxe4 4. Nxe4 Bf5 — Caro-Kann, Classical.",
     "Extremely solid defense. Black develops the light-squared bishop before playing e6. Used by Karpov and Anand."),
    ("WAG", "1. d4 Nf6 2. c4 e6 3. Nf3 b6 4. g3 Ba6 — Queen's Indian Defense.",
     "Hypermodern approach controlling the center with pieces. Strategically complex."),
]

WAVE2 = [
    ("HOWL", "After 1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O Be7 6.Re1 b5 7.Bb3 d6 8.c3 O-O 9.h3 Nb8 10.d4 Nbd7 — Breyer Variation.",
     "One of the deepest middlegame structures. The knight retreats to b8 to reroute to d7. World championship level."),
    ("WAG", "Najdorf Sicilian middlegame: White Be2, O-O, f4. Black a6, e5, Be7. English Attack setup.",
     "The sharpest branch of the sharpest opening. White attacks kingside, Black queenside."),
    ("GROWL", "White plays Rg1, h4, h5 without castling in the King's Indian. Premature kingside pawn storm.",
     "Aggressive but the king remains in the center. Double-edged, objectively dubious at high level."),
    ("BARK", "White moved the queen four times in the opening, lost two tempi. Black has completed development.",
     "Fundamental violation of opening principles. Moving same piece repeatedly while opponent develops."),
    ("HOWL", "King and pawn endgame: White Ke5, pawns d5 and e4. Black Ke7. Distant passed pawn and zugzwang.",
     "Fundamental endgame technique. The distant passed pawn forces the opposing king away. Pure calculation."),
    ("WAG", "Lucena position: White Kc7, Rd1, pawn c6. Black Ka7, Ra2. Bridge building technique.",
     "The most important rook endgame position. The bridge technique shields the king from checks."),
    ("GROWL", "Bishop endgame with wrong-colored rook pawn. White Kg6, Bc1, h6. Black Kh8.",
     "Drawn despite material advantage. Bishop cannot control h8, defending king sits in corner. Stalemate fortress."),
    ("WAG", "Philidor position: defender keeps rook on 6th rank, switches to back rank when pawn advances.",
     "Textbook defensive technique known since 1777. Passive defense works because rook prevents king advance."),
    ("BARK", "Playing for stalemate with K+R vs K+R+2P — but position has no stalemate resources.",
     "Hoping for a miracle. Without stalemate tricks, pure desperation with no theoretical basis."),
    ("HOWL", "Triangulation: White Kd3 plays Kd2-Ke2-Kd3 to transfer the move to Black. Zugzwang forces Black to concede.",
     "Advanced endgame concept. Extra tempo gained by triangulation creates zugzwang. Pure geometric logic."),
]

WAVE3 = [
    ("HOWL", "The Immortal Game (Anderssen 1851): 1.e4 e5 2.f4 exf4 3.Bc4 Qh4+ 4.Kf1 — sacrificing both rooks and bishop for a king hunt.",
     "One of the most famous games ever played. Brilliancy through sacrificial attack. Romantic chess at its peak."),
    ("HOWL", "Kasparov vs Topalov 1999: Rook sacrifice on d4 followed by devastating king hunt with every piece participating.",
     "Deep combination with 24.Rxd4! calculated over 15 moves. Pinnacle of human tactical vision."),
    ("WAG", "Fischer vs Spassky Game 6, 1972: Queen's Gambit Declined. Fischer's flawless positional masterpiece.",
     "Perfect strategic play. Every move improves position slightly. Cumulative advantage becomes overwhelming."),
    ("GROWL", "Fool's Mate variations: 1.g4 e5 2.f3 Qh4# — the shortest possible chess game.",
     "Represents the absolute worst play by White. Two-move checkmate."),
    ("WAG", "Berlin Defense wall: 1.e4 e5 2.Nf3 Nc6 3.Bb5 Nf6 4.O-O Nxe4 5.d4 Nd6 6.Bxc6 dxc6 7.dxe5 Nf5 8.Qxd8+ Kxd8.",
     "Kramnik's weapon vs Kasparov. Theoretically drawn endgame but extremely difficult to play."),
    ("GROWL", "Scandinavian Defense: 1.e4 d5 2.exd5 Qxd5 3.Nc3 Qa5 — but then failing to develop pieces.",
     "Opening is playable but common amateur mistakes compound. Queen wanders while development stalls."),
    ("BARK", "Random moves: 1.a3 h5 2.Ra2 Rh6 3.Ra1 Rh8 — both sides making meaningless rook shuffles.",
     "Aimless play. No center control, no development, no plan. Antithesis of chess strategy."),
    ("WAG", "Catalan Opening: 1.d4 Nf6 2.c4 e6 3.g3 d5 4.Bg2 — fianchetto in Queen's Gambit structure.",
     "Modern positional approach. Bishop on g2 pressures long diagonal. Favored by modern elite."),
    ("HOWL", "Capablanca endgame technique: simplification from slightly better middlegame into winning endgame through exchanges.",
     "The essence of classical chess. Convert small positional advantage through technique. Pure understanding."),
    ("BARK", "Opening with 1.Na3 followed by 2.Nb1, moving the knight back to its starting square.",
     "Complete waste of tempo. Knight achieves nothing and returns home. Equivalent to passing both moves."),
]

def judge(content: str, context: str, crystals: bool) -> dict:
    """Call POST /judge and return parsed response."""
    payload = json.dumps({
        "content": content,
        "context": context,
        "domain": "chess",
        "crystals": crystals,
    }).encode()
    req = urllib.request.Request(
        f"{BASE}/judge",
        data=payload,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {API_KEY}",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            return json.loads(resp.read())
    except Exception as e:
        return {"verdict": "ERROR", "q_score": {"total": 0.0}, "dogs_used": str(e)}

def tier_check(tier: str, q: float) -> bool:
    if tier == "HOWL":  return q > 0.528
    if tier == "WAG":   return 0.236 < q <= 0.618
    if tier == "GROWL": return 0.1 < q < 0.528
    if tier == "BARK":  return q <= 0.382
    return False

def main():
    wave = int(sys.argv[1]) if len(sys.argv) > 1 else 1

    stimuli = list(WAVE1)
    if wave >= 2: stimuli += WAVE2
    if wave >= 3: stimuli += WAVE3

    n = len(stimuli)

    # Get current Dogs
    try:
        req = urllib.request.Request(f"{BASE}/dogs", headers={"Authorization": f"Bearer {API_KEY}"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            dogs = json.loads(resp.read())
        dogs_str = ", ".join(dogs)
    except Exception:
        dogs_str = "unknown"

    print(f"\u25b6 Chess Bench \u2014 Wave {wave} ({n} stimuli \u00d7 2 modes = {n*2} calls)")
    print(f"  Dogs: {dogs_str}")
    print()

    results_on = []
    results_off = []
    passes = 0
    fails = 0
    t0 = time.time()

    for i, (tier, content, context) in enumerate(stimuli):
        label = content[:50]
        sys.stdout.write(f"  {i+1:2d}/{n} {tier:<6s} {label:<50s} ")
        sys.stdout.flush()

        v_on = judge(content, context, True)
        q_on = v_on["q_score"]["total"]
        vd_on = v_on["verdict"]

        v_off = judge(content, context, False)
        q_off = v_off["q_score"]["total"]
        vd_off = v_off["verdict"]

        delta = q_on - q_off
        results_on.append(q_on)
        results_off.append(q_off)

        ok = tier_check(tier, q_on)
        if ok: passes += 1
        else: fails += 1
        check = "OK" if ok else "MISS"

        dogs_used = v_on.get("dogs_used", "?")
        dog_count = len(dogs_used.split("+")) if "+" in dogs_used else 1

        print(f"ON:{vd_on:<5s} {q_on:.3f}  OFF:{vd_off:<5s} {q_off:.3f}  \u0394={delta:+.3f}  [{check}] ({dog_count}d)")

    elapsed = time.time() - t0

    # Summary
    print()
    deltas = [a - b for a, b in zip(results_on, results_off)]
    mean_on = sum(results_on) / n
    mean_off = sum(results_off) / n
    mean_d = sum(deltas) / n
    max_d = max(deltas)
    min_d = min(deltas)

    print(f"=== SUMMARY ({elapsed:.0f}s) ===")
    print(f"  Stimuli: {n}  |  Tier match: {passes}/{n}  |  Tier miss: {fails}/{n}")
    print(f"  Mean Q (ON): {mean_on:.4f}  Mean Q (OFF): {mean_off:.4f}")
    print(f"  Mean \u0394: {mean_d:+.4f}  Max \u0394: {max_d:+.4f}  Min \u0394: {min_d:+.4f}")

    # Crystal stats
    try:
        req = urllib.request.Request(f"{BASE}/crystals", headers={"Authorization": f"Bearer {API_KEY}"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read())
        crystals = data.get("crystals", data) if isinstance(data, dict) else data
        chess = [c for c in crystals if c.get("domain") == "chess"]
        states = {}
        for c in chess:
            s = c.get("state", "unknown")
            states[s] = states.get(s, 0) + 1
        print(f"  Chess crystals: {len(chess)}")
        for s, cnt in sorted(states.items()):
            print(f"    {s}: {cnt}")
    except Exception as e:
        print(f"  Crystal stats failed: {e}")

    print()
    print("Done.")
    sys.exit(1 if fails > n // 2 else 0)

if __name__ == "__main__":
    main()
