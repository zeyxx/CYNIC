#!/usr/bin/env python3
import urllib.request
import json
import time
import sys
import argparse

RPC_URLS = [
    "https://api.mainnet-beta.solana.com",
    "https://rpc.ankr.com/solana",
    "https://solana-rpc.publicnode.com"
]

def rpc_call(method, params):
    data = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    }
    for url in RPC_URLS:
        try:
            req = urllib.request.Request(url, json.dumps(data).encode('utf-8'), {'Content-Type': 'application/json'})
            with urllib.request.urlopen(req, timeout=10) as response:
                result = json.loads(response.read().decode())
                return result
        except Exception:
            continue
    return None

def main():
    parser = argparse.ArgumentParser(description="Solana Token Info Utility")
    parser.add_argument("token_address", help="The Solana token address")
    parser.add_argument("--check-owner", action="append", help="Check specific owner accounts")
    args = parser.parse_args()

    token = args.token_address

    print(f"--- Token Info for {token} ---")
    info = rpc_call("getAccountInfo", [token, {"encoding": "jsonParsed"}])
    print(json.dumps(info, indent=2))

    print("\n--- Largest Accounts ---")
    largest = rpc_call("getTokenLargestAccounts", [token])
    print(json.dumps(largest, indent=2))

    print("\n--- Token Age ---")
    sigs = rpc_call("getSignaturesForAddress", [token, {"limit": 1000}])
    if sigs and 'result' in sigs and sigs['result']:
        last_sig = sigs['result'][-1]
        block_time = last_sig.get('blockTime')
        if block_time:
            age_days = (time.time() - block_time) / 86400
            print(f"Age in days: {age_days:.2f}")
            print(f"Creation time: {time.ctime(block_time)}")
    
    if args.check_owner:
        print("\n--- Owner Accounts Info ---")
        for owner in args.check_owner:
            owner_info = rpc_call("getAccountInfo", [owner, {"encoding": "jsonParsed"}])
            print(f"{owner}:")
            print(json.dumps(owner_info, indent=2))

if __name__ == "__main__":
    main()
