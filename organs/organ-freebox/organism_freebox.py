import json
import os
import time
import requests
import hmac
import hashlib

FREEBOX_URL = "http://mafreebox.freebox.fr/api/v8"
APP_ID = "fr.cynic.organ_freebox"
APP_NAME = "CYNIC Organ Freebox"
APP_VERSION = "1.0.0"
DEVICE_NAME = "CYNIC Kernel"

TOKEN_FILE = os.path.expanduser("~/.freebox_token")

def authorize():
    print("Demande d'autorisation à la Freebox...")
    payload = {
        "app_id": APP_ID,
        "app_name": APP_NAME,
        "app_version": APP_VERSION,
        "device_name": DEVICE_NAME
    }
    r = requests.post(f"{FREEBOX_URL}/login/authorize/", json=payload)
    if not r.ok:
        print("Erreur:", r.text)
        return
    data = r.json()
    if not data["success"]:
        print("Echec:", data)
        return
    
    app_token = data["result"]["app_token"]
    track_id = data["result"]["track_id"]
    print("\n👉 VEUILLEZ VALIDER SUR L'ECRAN DE LA FREEBOX (Fleche Droite / Oui)")
    
    while True:
        r2 = requests.get(f"{FREEBOX_URL}/login/authorize/{track_id}")
        status = r2.json()["result"]["status"]
        if status == "granted":
            print("✅ Autorisation accordée !")
            with open(TOKEN_FILE, "w") as f:
                json.dump({"app_token": app_token}, f)
            break
        elif status != "pending":
            print("❌ Autorisation refusée ou expirée :", status)
            break
        time.sleep(2)

def login():
    if not os.path.exists(TOKEN_FILE):
        authorize()
        if not os.path.exists(TOKEN_FILE):
            return None
            
    with open(TOKEN_FILE, "r") as f:
        app_token = json.load(f)["app_token"]
        
    r = requests.get(f"{FREEBOX_URL}/login/")
    challenge = r.json()["result"]["challenge"]
    
    password = hmac.new(app_token.encode(), challenge.encode(), hashlib.sha1).hexdigest()
    
    r2 = requests.post(f"{FREEBOX_URL}/login/session/", json={
        "app_id": APP_ID,
        "password": password
    })
    
    return r2.json()["result"]["session_token"]

def open_port(session_token):
    headers = {"X-Fbx-App-Auth": session_token}
    payload = {
        "enabled": True,
        "ip": "192.168.0.37",
        "wan_port_start": 443,
        "wan_port_end": 443,
        "lan_port": 443,
        "lan_ip": "192.168.0.37",
        "src_ip": "0.0.0.0",
        "ip_proto": "tcp",
        "comment": "CYNIC Cloud Gateway HTTPS"
    }
    r = requests.post(f"{FREEBOX_URL}/fw/redir/", json=payload, headers=headers)
    print("Port opening response:", r.text)

if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1 and sys.argv[1] == "auth":
        authorize()
    else:
        token = login()
        if token:
            open_port(token)
