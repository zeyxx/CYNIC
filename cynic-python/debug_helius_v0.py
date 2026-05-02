
import os
import requests
import json

def debug_helius_v0():
    api_key = "ac94987a-2acd-4778-8759-1bb4708e905b"
    wallet = "7Cdybf9okQYqy8MJzoKbgsgf4AoaMtAEyU15hQDSKQyL" # WIF Whale
    
    url = f"https://api.helius.xyz/v0/addresses/{wallet}/transactions?api-key={api_key}"
    print(f"Calling v0 enhanced transactions for {wallet}...")
    resp = requests.get(url)
    print(f"Status: {resp.status_code}")
    if resp.status_code == 200:
        data = resp.json()
        print(f"Found {len(data)} transactions")
        if len(data) > 0:
            tx = data[0]
            print(f"Transaction keys: {list(tx.keys())}")
            if "instructions" in tx:
                print(f"Instructions: {len(tx['instructions'])}")
            if "type" in tx:
                print(f"Type: {tx['type']}")

if __name__ == "__main__":
    debug_helius_v0()
