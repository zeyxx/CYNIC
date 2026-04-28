#!/usr/bin/env python3
"""
CYNIC High-Signal Judge Submitter
Submits novel high-signal tweets to cynic_judge
"""
import json
import re
from datetime import datetime

# Load dataset
with open('captures/dataset.jsonl', 'r') as f:
    tweets = [json.loads(line) for line in f if line.strip()]

# Filter high-signal (>=3)
high_signal = [t for t in tweets if t.get('signal_score', 0) >= 3]
high_signal.sort(key=lambda x: x.get('signal_score', 0), reverse=True)

# Known bot/scam accounts
bot_accounts = {
    'EP_PETER', 'ArthurBomb', 'EMCruzzz', 'jeffersonyoan', 'Njabulobhabha',
    'iksLurpee', 'CinisoMasilela', 'husseinalshrafy', 'ArishnaP', 'SavannahHinkley',
    'jrjr1014', 'UgochukwuE73862', 'kristenmyvida', 'SamyHaloui', 'raulabeso',
    'EnaniKamal', 'Tytina0519'
}
scam_accounts = {'Gary_Recovery_', 'thelipglossguy_', 'assertguard'}

def is_bot_account(author):
    if not author:
        return True
    screen_name = author.lower()
    for bot in bot_accounts:
        if bot.lower() in screen_name:
            return True
    for scam in scam_accounts:
        if scam.lower() in screen_name:
            return True
    return False

def extract_tokens(text):
    return re.findall(r'\$([A-Z]{3,10})', text)

# Already judged signals (from handoff and current session)
already_judged = {
    'gcrtrd_BEDROCK_ASDFASDFA_Exactly as predicted'  # $BEDROCK rug confirmed
}

# Collect judgments to submit
judgments = []
observations = []

for i, tweet in enumerate(high_signal[:100], 1):  # Process top 100
    author = tweet.get('author_screen_name', 'unknown')
    score = tweet.get('signal_score', 0)
    narratives = tweet.get('narratives', [])
    cashtags = tweet.get('cashtags', [])
    text = tweet.get('text', '')[:500]
    
    # Skip known bots/scams
    if is_bot_account(author):
        print(f"#{i} SKIPPED (bot): @{author}")
        continue
    
    signal_id = f"{author}_{cashtags}_{text[:50]}"
    
    # Skip already judged
    if signal_id in already_judged:
        print(f"#{i} ALREADY JUDGED: {signal_id}")
        continue
    
    # Determine action based on criteria
    worth_judging = (
        score >= 4 and
        (len(narratives) >= 2 or 'rug_warning' in narratives or len(cashtags) >= 2)
    )
    
    if worth_judging:
        # Check for falsifiable claims about tokens
        has_falsifiable_claim = (
            'rug' in text.lower() or
            'scam' in text.lower() or
            'safe' in text.lower() or
            'warning' in text.lower() or
            any(c in text.lower() for c in ['fee', 'token', 'lock', 'dead'])
        )
        
        is_bot_or_coordination = False
        # Check if coordinated (same content from multiple accounts)
        # For now, assume novel if author is not in bot list
        
        if has_falsifiable_claim and not is_bot_or_coordination:
            judgments.append({
                'order': len(judgments) + 1,
                'signal_id': signal_id,
                'score': score,
                'author': author,
                'narratives': narratives,
                'tokens': cashtags,
                'text': text
            })
            print(f"#{i} JUDGE: @{author} (score={score}, narr={narratives}, tokens={cashtags})")
        else:
            observations.append({
                'order': len(observations) + 1,
                'signal_id': signal_id,
                'score': score,
                'author': author,
                'narratives': narratives,
                'text': text
            })
            print(f"#{i} OBSERVE: @{author} (score={score})")
    else:
        observations.append({
            'order': len(observations) + 1,
            'signal_id': signal_id,
            'score': score,
            'author': author,
            'narratives': narratives,
            'text': text
        })
        print(f"#{i} OBSERVE: @{author} (score={score})")

print(f"\n=== SUMMARY ===")
print(f"Judgments to submit: {len(judgments)}")
print(f"Observations to record: {len(observations)}")

# Print judgments in detail
print("\n=== JUDGMENTS TO SUBMIT ===\n")
for j in judgments:
    print(f"JUDGMENT #{j['order']}")
    print(f"Author: @{j['author']}")
    print(f"Score: {j['score']}")
    print(f"Narratives: {j['narratives']}")
    print(f"Tokens: {j['tokens']}")
    print(f"Text: {j['text'][:300]}")
    print()
