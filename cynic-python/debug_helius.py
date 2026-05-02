
import os
import requests
import json

def debug_helius():
    api_key = "ac94987a-2acd-4778-8759-1bb4708e905b"
    wallet = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" # USDC
    
    # 3. REST call history
    rest_url = f"https://api.helius.xyz/v1/wallet/{wallet}/history?api-key={api_key}&limit=5"
    print(f"\nCalling REST history for {wallet}...")
    resp = requests.get(rest_url)
    print(f"Status: {resp.status_code}")
    data = resp.json()
    print(f"Keys in response: {list(data.keys())}")
    if isinstance(data, list):
        print(f"Response is a list with {len(data)} items")
    elif "transactions" in data:
        print(f"Found 'transactions' with {len(data['transactions'])} items")
    else:
        print(f"Raw response head: {str(data)[:200]}")

if __name__ == "__main__":
    debug_helius()
