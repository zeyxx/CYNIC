
import os
import sys
import json
from dataclasses import asdict
from wallet_corpus_builder import build_corpus
from wallet_behavior_helius import HeliusWalletCollector
from wallet_behavior_validator import WalletValidator

def get_helius_key():
    # 1. Try environment
    key = os.getenv("HELIUS_API_KEY")
    if key and len(key) >= 36:
        return key
        
    # 2. Try ~/.helius/config.json
    config_path = os.path.expanduser("~/.helius/config.json")
    if os.path.exists(config_path):
        try:
            with open(config_path) as f:
                data = json.load(f)
                key = data.get("apiKey")
                if key:
                    return key
        except:
            pass
            
    # 3. Try ~/.cynic-env with fix
    env_path = os.path.expanduser("~/.cynic-env")
    if os.path.exists(env_path):
        with open(env_path) as f:
            for line in f:
                if "HELIUS_API_KEY=" in line:
                    key = line.split("=")[1].strip().strip('"').replace("export ", "")
                    if len(key) == 35: # Missing 'a'
                        key = "a" + key
                    return key
    return None

def main():
    key = get_helius_key()
    if not key:
        print("Error: HELIUS_API_KEY not found in environment, ~/.helius/config.json, or ~/.cynic-env")
        sys.exit(1)
    
    os.environ["HELIUS_API_KEY"] = key
    print(f"Using Helius API Key: {key[:4]}...{key[-4:]}")

    print("=== CYNIC: Real Corpus Collection & Validation ===")
    
    # 1. Collect
    collector = HeliusWalletCollector()
    corpus_file = "real_wallet_corpus_validated.json"
    print(f"\n1. Collecting behavior profiles to {corpus_file}...")
    try:
        build_corpus(collector, output_file=corpus_file)
    except Exception as e:
        print(f"Error during collection: {e}")
        if not os.path.exists(corpus_file):
             sys.exit(1)
        print("Continuing with existing partial corpus file...")
    
    # 2. Validate
    print(f"\n2. Running validation on {corpus_file}...")
    try:
        profiles, labels = WalletValidator.load_corpus(corpus_file)
        if not profiles:
            print("Error: No profiles loaded from corpus.")
            sys.exit(1)
            
        result = WalletValidator.validate(profiles, labels)
        
        print("\n=== Validation Results ===")
        print(result)
        
        # Output as JSON for machine reading
        results_json = asdict(result)
        with open("validation_results.json", "w") as f:
            json.dump(results_json, f, indent=2)
            
        if result.roc_auc > 0.7:
            print("\n✅ Falsification Test 2 PASSED (ROC-AUC > 0.7)")
        else:
            print("\n❌ Falsification Test 2 FAILED (ROC-AUC <= 0.7)")
    except Exception as e:
        print(f"Error during validation: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
