#!/usr/bin/env python3
import sys
import json
from pathlib import Path
import os

try:
    import browser_cookie3
except ImportError:
    import subprocess
    subprocess.run([sys.executable, "-m", "pip", "install", "browser-cookie3"])
    import browser_cookie3

def export_cookies():
    print("Extraction des cookies depuis Google Chrome (hôte)...")
    try:
        cj = browser_cookie3.chrome(domain_name=".twitter.com")
        cj2 = browser_cookie3.chrome(domain_name=".x.com")
    except Exception as e:
        print(f"Erreur d'extraction: {e}")
        return

    pw_cookies = []
    for cookie in list(cj) + list(cj2):
        pw_cookie = {
            "name": cookie.name,
            "value": cookie.value,
            "domain": cookie.domain,
            "path": cookie.path,
            "secure": bool(cookie.secure),
            "sameSite": "Lax"
        }
        if cookie.expires:
            pw_cookie["expires"] = float(cookie.expires)
        pw_cookies.append(pw_cookie)

    if not pw_cookies:
        print("Aucun cookie X/Twitter trouvé.")
        return

    out_file = "/tmp/x_cookies.json"
    with open(out_file, "w") as f:
        json.dump(pw_cookies, f)
    
    os.chmod(out_file, 0o777)
    print(f"Succès ! {len(pw_cookies)} cookies exportés dans {out_file}.")

if __name__ == "__main__":
    export_cookies()
