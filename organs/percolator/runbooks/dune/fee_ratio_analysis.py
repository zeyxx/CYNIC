import json
from pathlib import Path

ROOT = next(path for path in Path(__file__).resolve().parents if (path / "axioms").is_dir())

with (ROOT / "axioms/pumpfun_fee_tiers.json").open("r", encoding="utf-8") as f:
    data = json.load(f)

print("| Tier | Max Cap (SOL) | Creator BPS | Protocol BPS | LP BPS | Ratio Protocol/Creator | Ratio Protocol/Creator (%) |")
print("|------|---------------|-------------|--------------|--------|------------------------|-----------------------------|")

for i, tier in enumerate(data['fee_tiers']):
    creator = tier['fees']['creator_fee_bps']
    protocol = tier['fees']['protocol_fee_bps']
    lp = tier['fees']['lp_fee_bps']
    
    threshold = tier['market_cap_lamports_threshold_max']
    if threshold is None:
        cap_sol = "Infinite"
    else:
        cap_sol = f"{threshold / 10**9:,.0f}"
        
    ratio = protocol / creator
    ratio_pct = ratio * 100
    
    print(f"| {i+1} | {cap_sol} | {creator} | {protocol} | {lp} | {ratio:.2f}x | {ratio_pct:.1f}% |")
