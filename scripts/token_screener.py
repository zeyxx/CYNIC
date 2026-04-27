#!/usr/bin/env python3
"""
CYNIC Token Screener — fetch on-chain data, build stimulus, submit to kernel.

The kernel judges. This script feeds.

Usage:
    python3 scripts/token_screener.py <MINT_ADDRESS>
    python3 scripts/token_screener.py <MINT_ADDRESS> --dry-run   # print stimulus, don't submit
    python3 scripts/token_screener.py <MINT_ADDRESS> --verbose    # show raw API responses

Data sources:
    - Helius DAS getAsset: metadata, authorities, price, supply
    - Solana RPC getTokenLargestAccounts: top-20 holders (via Helius endpoint)

Pool filtering: DEX program addresses from HolDex (sollama58/HolDex).
"""

import json
import os
import sys
import urllib.request
import urllib.error
from pathlib import Path

# ── Helius config ──

def load_helius_key() -> str:
    """Load Helius API key from env or ~/.helius/config.json."""
    key = os.environ.get("HELIUS_API_KEY")
    if key:
        return key
    config_path = Path.home() / ".helius" / "config.json"
    if config_path.exists():
        with open(config_path) as f:
            data = json.load(f)
            return data.get("apiKey", "")
    print("ERROR: No HELIUS_API_KEY env var and no ~/.helius/config.json", file=sys.stderr)
    sys.exit(1)


# ── DEX pool addresses (from HolDex sollama58/HolDex kScoreUpdater.js) ──

DEX_PROGRAMS = {
    # Raydium
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
    "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h",
    # Orca
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
    # Meteora
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    # OpenBook/Serum
    "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",
    "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
    "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb",
    # PumpFun
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
    "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
}

KNOWN_POOL_WALLETS = {
    "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM",  # PumpFun Fee
}

# Solana system/burn addresses
BURN_ADDRESSES = {
    "11111111111111111111111111111111",
    "1nc1nerator11111111111111111111111111111111",
}


# ── Helius API calls ──

def helius_rpc(api_key: str, method: str, params: list, retries: int = 3) -> dict | None:
    """Call Solana RPC via Helius endpoint with retry on transient errors."""
    import time
    url = f"https://mainnet.helius-rpc.com/?api-key={api_key}"
    body = json.dumps({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    }).encode()
    for attempt in range(retries):
        req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.load(resp)
                if "error" in data:
                    err = data["error"]
                    # Retry on transient "overloaded" errors
                    if attempt < retries - 1 and "overloaded" in str(err.get("message", "")):
                        print(f"RPC transient ({method}), retry {attempt+1}...", file=sys.stderr)
                        time.sleep(2 ** attempt)
                        continue
                    print(f"RPC error ({method}): {err}", file=sys.stderr)
                    return None
                return data.get("result")
        except Exception as e:
            if attempt < retries - 1:
                time.sleep(2 ** attempt)
                continue
            print(f"RPC call failed ({method}): {e}", file=sys.stderr)
            return None
    return None


def helius_das(api_key: str, method: str, params: dict) -> dict | None:
    """Call Helius DAS API (JSON-RPC over REST)."""
    url = f"https://mainnet.helius-rpc.com/?api-key={api_key}"
    body = json.dumps({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    }).encode()
    req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.load(resp)
            if "error" in data:
                print(f"DAS error ({method}): {data['error']}", file=sys.stderr)
                return None
            return data.get("result")
    except Exception as e:
        print(f"DAS call failed ({method}): {e}", file=sys.stderr)
        return None


# ── Data extraction ──

def fetch_asset(api_key: str, mint: str) -> dict:
    """Fetch token metadata via Helius DAS getAsset."""
    result = helius_das(api_key, "getAsset", {"id": mint})
    if not result:
        return {}

    info = {}
    # Content metadata
    content = result.get("content", {})
    metadata = content.get("metadata", {})
    info["name"] = metadata.get("name")
    info["symbol"] = metadata.get("symbol")
    info["token_standard"] = metadata.get("token_standard")

    # Token info
    token_info = result.get("token_info", {})
    info["supply"] = token_info.get("supply")
    info["decimals"] = token_info.get("decimals")
    info["mint_authority"] = token_info.get("mint_authority")
    info["freeze_authority"] = token_info.get("freeze_authority")

    # Price
    price_info = token_info.get("price_info", {})
    info["price_usd"] = price_info.get("price_per_token")

    # Mutability
    info["mutable"] = result.get("mutable")

    # Description (from off-chain metadata if present)
    json_uri = content.get("json_uri", "")
    info["description"] = metadata.get("description")

    return info


def estimate_maturity_tier(asset: dict, concentration: dict) -> str:
    """Derive maturity tier from on-chain signals already available.
    No extra API calls — uses data from getAsset + concentration analysis.
    Returns a human-readable maturity string for the stimulus."""
    revoked_mint = not asset.get("mint_authority")
    revoked_freeze = not asset.get("freeze_authority")
    immutable = asset.get("mutable") is False
    hhi = concentration.get("herfindahl")
    mcap = 0
    supply = asset.get("supply", 0)
    decimals = asset.get("decimals", 0)
    price = asset.get("price_usd", 0)
    if supply and decimals is not None and price:
        mcap = (supply / (10 ** decimals)) * price

    # Tier logic — derived from observable signals, no hardcoded age
    # For fungible tokens, metadata mutability is normal (UI updates).
    # Weight immutability as a bonus, not a requirement.
    is_fungible = asset.get("token_standard") in ("Fungible", "FungibleAsset", None)
    signals_strong = sum([
        revoked_mint,
        revoked_freeze,
        immutable or (is_fungible and revoked_mint and revoked_freeze),  # fungibles: authorities matter more than metadata
        hhi is not None and hhi < 0.15,
        mcap > 100_000_000,
    ])
    signals_moderate = sum([
        revoked_mint or revoked_freeze,
        mcap > 1_000_000,
        hhi is not None and hhi < 0.40,
    ])

    if signals_strong >= 4:
        return "ESTABLISHED (authorities revoked, distributed holders, high market cap — eligible for full score range)"
    elif signals_moderate >= 2:
        return "MODERATE (some positive signals, building track record — max axiom score ~0.55)"
    else:
        return "NEW/UNPROVEN (few positive signals, high caution — max axiom score ~0.45)"


def fetch_largest_accounts(api_key: str, mint: str) -> list[dict]:
    """Fetch top-20 largest token accounts via Solana RPC."""
    result = helius_rpc(api_key, "getTokenLargestAccounts", [mint])
    if not result or "value" not in result:
        return []
    return result["value"]


def resolve_pool_owners(api_key: str, accounts: list[dict]) -> set[str]:
    """
    Resolve which token accounts are DEX pools.
    Two-step: token account → wallet owner → check wallet's owning program.
    (Pattern from HolDex sollama58/HolDex kScoreUpdater.js batchCheckPools)

    Returns set of token account addresses that are pools.
    """
    if not accounts:
        return set()

    # Step 1: Get wallet owners from token accounts
    token_addrs = [a.get("address", "") for a in accounts]
    result = helius_rpc(api_key, "getMultipleAccounts", [token_addrs, {"encoding": "jsonParsed"}])
    if not result or "value" not in result:
        return set()

    wallet_to_token = {}  # wallet_addr -> token_account_addr
    for i, info in enumerate(result["value"]):
        if not info:
            continue
        parsed = info.get("data", {}).get("parsed", {}).get("info", {})
        wallet_owner = parsed.get("owner", "")
        if wallet_owner:
            wallet_to_token[wallet_owner] = token_addrs[i]

    if not wallet_to_token:
        return set()

    # Quick check: known pool wallets and burn addresses
    pool_token_addrs = set()
    wallets_to_check = []
    for wallet, token_addr in wallet_to_token.items():
        if wallet in KNOWN_POOL_WALLETS or wallet in BURN_ADDRESSES:
            pool_token_addrs.add(token_addr)
        else:
            wallets_to_check.append((wallet, token_addr))

    if not wallets_to_check:
        return pool_token_addrs

    # Step 2: Check what PROGRAM owns each wallet (PDA → DEX program?)
    wallet_addrs = [w for w, _ in wallets_to_check]
    result2 = helius_rpc(api_key, "getMultipleAccounts", [wallet_addrs, {"encoding": "base64"}])
    if not result2 or "value" not in result2:
        return pool_token_addrs

    for i, info in enumerate(result2["value"]):
        if not info:
            continue
        owner_program = info.get("owner", "")
        if owner_program in DEX_PROGRAMS:
            pool_token_addrs.add(wallets_to_check[i][1])

    return pool_token_addrs


def compute_concentration(api_key: str, accounts: list[dict], total_supply: int) -> dict:
    """
    Compute holder concentration from top accounts.
    Resolves wallet owners via getMultipleAccounts, then filters DEX pools.

    Returns: {top1_pct, top10_pct, herfindahl, filtered_count, pool_count}
    """
    if not accounts or total_supply == 0:
        return {"top1_pct": None, "top10_pct": None, "herfindahl": None,
                "filtered_count": 0, "pool_count": 0}

    # Resolve pool owners (1 batch RPC call)
    pool_addrs = resolve_pool_owners(api_key, accounts)

    # Filter pools and burns
    real_holders = []
    pool_count = len(pool_addrs)
    for acc in accounts:
        addr = acc.get("address", "")
        if addr in pool_addrs:
            continue
        amount = int(acc.get("amount", "0"))
        if amount > 0:
            real_holders.append(amount)

    if not real_holders:
        return {"top1_pct": None, "top10_pct": None, "herfindahl": None,
                "filtered_count": 0, "pool_count": pool_count}

    # Sort descending (should already be sorted, but be safe)
    real_holders.sort(reverse=True)

    top1_pct = (real_holders[0] / total_supply) * 100 if real_holders else 0
    top10_sum = sum(real_holders[:10])
    top10_pct = (top10_sum / total_supply) * 100

    # Herfindahl-Hirschman Index on available accounts
    hhi = sum((bal / total_supply) ** 2 for bal in real_holders)

    return {
        "top1_pct": round(top1_pct, 2),
        "top10_pct": round(top10_pct, 2),
        "herfindahl": round(hhi, 4),
        "filtered_count": len(real_holders),
        "pool_count": pool_count,
    }


# ── Stimulus builder (mirrors stimulus.rs::build_token_stimulus) ──

def build_stimulus(mint: str, asset: dict, concentration: dict) -> str:
    """Build structured stimulus for CYNIC Dogs."""
    lines = ["[DOMAIN: token-analysis]", "", "[METRICS]"]
    lines.append(f"mint: {mint}")

    if asset.get("name"):
        lines.append(f"name: {asset['name']}")
    if asset.get("symbol"):
        lines.append(f"symbol: {asset['symbol']}")

    # Supply & market cap
    supply_raw = asset.get("supply")
    decimals = asset.get("decimals")
    price = asset.get("price_usd")
    if supply_raw is not None and decimals is not None:
        human_supply = supply_raw / (10 ** decimals)
        lines.append(f"supply: {human_supply:,.0f}")
        if price:
            mcap = human_supply * price
            lines.append(f"price_usd: ${price:.6f}")
            lines.append(f"market_cap: ${mcap:,.0f}")

    # Concentration (post-filter)
    if concentration.get("top1_pct") is not None:
        lines.append(f"top_1_wallet_pct: {concentration['top1_pct']}%")
    if concentration.get("top10_pct") is not None:
        lines.append(f"top_10_wallets_pct: {concentration['top10_pct']}%")
    if concentration.get("herfindahl") is not None:
        lines.append(f"herfindahl_index: {concentration['herfindahl']}")
    if concentration.get("pool_count", 0) > 0:
        lines.append(f"dex_pools_filtered: {concentration['pool_count']}")

    # Authorities
    mint_auth = asset.get("mint_authority")
    freeze_auth = asset.get("freeze_authority")
    lines.append(f"mint_authority: {'ACTIVE (can mint more tokens)' if mint_auth else 'REVOKED (supply is fixed)'}")
    lines.append(f"freeze_authority: {'ACTIVE (can freeze wallets)' if freeze_auth else 'REVOKED (wallets are free)'}")

    # Mutability
    if asset.get("mutable") is not None:
        lines.append(f"metadata_mutable: {'YES (can change name/symbol)' if asset['mutable'] else 'NO (immutable)'}")

    # Token standard
    if asset.get("token_standard"):
        lines.append(f"token_standard: {asset['token_standard']}")

    # Description
    if asset.get("description"):
        desc = asset["description"][:200]
        lines.append(f"description: {desc}")

    # Maturity tier — derived from on-chain signals, drives HOWL/AMB discrimination
    maturity = asset.get("maturity_tier")
    if maturity:
        lines.append(f"maturity_assessment: {maturity}")

    # Baselines
    lines.append("")
    lines.append("[BASELINES]")
    lines.append("healthy_token: holders>100, top_1<15%, herfindahl<0.15, age>30d, mint_authority=revoked, lp=burned")
    lines.append("moderate_risk: holders 20-100, top_1 15-40%, age 1-30d, lp=locked")
    lines.append("high_risk_rug: holders<20, top_1>50%, age<24h, mint_authority=active, lp=unsecured")
    lines.append("note: 98.6% of pump.fun tokens are rug pulls (Solidus Labs 2025). Baseline for new tokens is skepticism, not trust.")

    # Axiom evidence
    lines.append("")
    lines.append("[AXIOM EVIDENCE]")
    lines.append("FIDELITY: Does the on-chain state match what a legitimate project would show? Is the token faithful to its claimed purpose?")
    lines.append("PHI: Is the holder distribution proportional? Is the tokenomics structure harmonious or concentrated?")
    lines.append("VERIFY: Can these metrics be independently verified on-chain? Are there verifiable red flags or green flags?")
    lines.append("CULTURE: Does this token follow established Solana token standards? Is the authority model consistent with good practices?")
    lines.append("BURN: Is the token efficiently structured? Burned supply, minimal waste, no unnecessary authorities retained?")
    lines.append("SOVEREIGNTY: Is control distributed or concentrated? Can individual holders act freely without one wallet dominating?")

    # Question
    lines.append("")
    lines.append("[QUESTION]")
    lines.append("Based on the on-chain metrics above, evaluate this token's legitimacy and risk level. Score each axiom from 0.05 to 0.618.")

    return "\n".join(lines)


# ── CYNIC kernel submission ──

def submit_to_kernel(stimulus: str) -> dict | None:
    """POST stimulus to CYNIC /judge endpoint."""
    kernel_addr = os.environ.get("CYNIC_REST_ADDR", "")
    api_key = os.environ.get("CYNIC_API_KEY", "")

    if not kernel_addr:
        # Try loading from ~/.cynic-env
        env_file = Path.home() / ".cynic-env"
        if env_file.exists():
            for line in env_file.read_text().splitlines():
                line = line.strip()
                if line.startswith("#") or "=" not in line:
                    continue
                k, v = line.split("=", 1)
                k = k.strip()
                v = v.strip().strip('"').strip("'")
                if k == "CYNIC_REST_ADDR":
                    kernel_addr = v
                elif k == "CYNIC_API_KEY":
                    api_key = v

    if not kernel_addr:
        print("ERROR: CYNIC_REST_ADDR not set", file=sys.stderr)
        return None

    if not kernel_addr.startswith("http"):
        kernel_addr = f"http://{kernel_addr}"

    url = f"{kernel_addr}/judge"
    body = json.dumps({
        "content": stimulus,
        "domain": "token-analysis",
        "dogs": ["sovereign"],  # Use only local Dogs — preserve cloud quota for human/agent analysis
    }).encode()

    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    req = urllib.request.Request(url, data=body, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            return json.load(resp)
    except urllib.error.HTTPError as e:
        print(f"Kernel error {e.code}: {e.read().decode()}", file=sys.stderr)
        return None
    except Exception as e:
        print(f"Kernel call failed: {e}", file=sys.stderr)
        return None


# ── Main ──

def main():
    if len(sys.argv) < 2:
        print("Usage: token_screener.py <MINT_ADDRESS> [--dry-run] [--verbose]")
        sys.exit(1)

    mint = sys.argv[1]
    dry_run = "--dry-run" in sys.argv
    verbose = "--verbose" in sys.argv

    api_key = load_helius_key()

    # 1. Fetch data (parallel would be nice but stdlib doesn't need it — 2 calls, <2s)
    print(f"Fetching {mint[:8]}...", file=sys.stderr)
    asset = fetch_asset(api_key, mint)
    largest = fetch_largest_accounts(api_key, mint)
    # (age_days removed — rentEpoch unreliable for rent-exempt accounts)

    if verbose:
        print(f"\n--- getAsset ---\n{json.dumps(asset, indent=2)}", file=sys.stderr)
        print(f"\n--- getTokenLargestAccounts ({len(largest)} accounts) ---", file=sys.stderr)
        for acc in largest[:5]:
            print(f"  {acc['address'][:12]}... = {acc.get('uiAmountString', '?')}", file=sys.stderr)

    if not asset:
        print("ERROR: getAsset returned nothing — is this a valid token?", file=sys.stderr)
        sys.exit(1)

    # 2. Compute concentration (resolves pool owners via getMultipleAccounts)
    total_supply = asset.get("supply", 0)
    concentration = compute_concentration(api_key, largest, total_supply)

    if verbose:
        print(f"\n--- Concentration ---\n{json.dumps(concentration, indent=2)}", file=sys.stderr)

    # 3. Build stimulus
    asset["maturity_tier"] = estimate_maturity_tier(asset, concentration)
    stimulus = build_stimulus(mint, asset, concentration)

    if dry_run:
        print(stimulus)
        print(f"\n--- Stimulus: {len(stimulus)} chars, ~{len(stimulus)//4} tokens ---", file=sys.stderr)
        sys.exit(0)

    # 4. Submit to kernel
    print(f"Submitting to CYNIC...", file=sys.stderr)
    result = submit_to_kernel(stimulus)

    if result:
        print(json.dumps(result, indent=2))
    else:
        print("ERROR: No verdict received", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
