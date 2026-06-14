import urllib.request
import json
import urllib.error
import time

RPC_URL = "https://api.mainnet-beta.solana.com"
MINT = "FeMrhDeddUvuzo25RZriZD6Te8xy3tdyKLLXBhEEpump"

req = urllib.request.Request(
    RPC_URL,
    data=json.dumps({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenLargestAccounts",
        "params": [MINT]
    }).encode(),
    headers={"Content-Type": "application/json"}
)

try:
    time.sleep(2)
    with urllib.request.urlopen(req) as response:
        print(response.read().decode())
except urllib.error.HTTPError as e:
    print("Error:", e.code, e.read().decode())
