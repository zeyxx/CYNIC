
import os
import requests
import json

def debug_helius_tx():
    api_key = "ac94987a-2acd-4778-8759-1bb4708e905b"
    wallet = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" # USDC
    
    rest_url = f"https://api.helius.xyz/v1/wallet/{wallet}/history?api-key={api_key}&limit=1"
    resp = requests.get(rest_url)
    data = resp.json()
    if "data" in data and len(data["data"]) > 0:
        tx = data["data"][0]
        print(f"Transaction keys: {list(tx.keys())}")
        if "instructions" in tx:
            print(f"Instructions: {len(tx['instructions'])}")
            if len(tx['instructions']) > 0:
                print(f"First instruction keys: {list(tx['instructions'][0].keys())}")
                print(f"Program ID: {tx['instructions'][0].get('programId')}")
        else:
            print("No 'instructions' key in transaction!")
            # Maybe it's called something else?
            for k, v in tx.items():
                if isinstance(v, list) and len(v) > 0 and isinstance(v[0], dict):
                    print(f"Found other list of dicts: {k}")

if __name__ == "__main__":
    debug_helius_tx()
