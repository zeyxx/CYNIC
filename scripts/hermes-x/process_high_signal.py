#!/usr/bin/env python3
"""
CYNIC High-Signal Tweet Processor
Scans dataset.jsonl, identifies novel high-signal tweets, and submits to cynic_judge
"""
import json
import re
import sys
from datetime import datetime

# Load dataset
with open('captures/dataset.jsonl', 'r') as f:
    tweets = [json.loads(line) for line in f if line.strip()]

# Filter high-signal (>=3)
high_signal = [t for t in tweets if t.get('signal_score', 0) >= 3]
print(f'High-signal tweets (>=3): {len(high_signal)}')

# Group by signal_score descending
high_signal.sort(key=lambda x: x.get('signal_score', 0), reverse=True)

# Track processed signals to avoid duplicates
processed_signals = set()

# Known bot accounts (skip these)
bot_accounts = {
    'EP_PETER', 'ArthurBomb', 'EMCruzzz', 'jeffersonyoan', 'Njabulobhabha',
    'iksLurpee', 'CinisoMasilela', 'husseinalshrafy', 'ArishnaP', 'SavannahHinkley',
    'jrjr1014', 'UgochukwuE73862', 'kristenmyvida', 'SamyHaloui', 'raulabeso',
    'EnaniKamal', 'Tytina0519'
}

# Known scam accounts (skip)
scam_accounts = {'Gary_Recovery_', 'thelipglossguy_', 'assertguard'}

def is_bot_account(author):
    """Check if author is a known bot/scam account"""
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
    """Extract cashtags from text"""
    return re.findall(r'\$([A-Z]{3,10})', text)

def make_signal_id(tweet):
    """Create a unique ID for this signal based on content"""
    text = tweet.get('text', '')[:100]
    author = tweet.get('author_screen_name', 'unknown')
    cashtags = tweet.get('cashtags', [])
    return f"{author}_{cashtags}_{text[:50]}"

# Process top signals for judgment
print("\n=== HIGH-SIGNAL TWEETS FOR JUDGMENT ===\n")

for i, tweet in enumerate(high_signal[:50], 1):  # Process top 50
    author = tweet.get('author_screen_name', 'unknown')
    score = tweet.get('signal_score', 0)
    narratives = tweet.get('narratives', [])
    cashtags = tweet.get('cashtags', [])
    text = tweet.get('text', '')[:300]
    
    # Skip known bots/scams
    if is_bot_account(author):
        print(f"#{i} SKIPPED (bot account): @{author}")
        continue
    
    # Create signal ID
    signal_id = make_signal_id(tweet)
    
    # Check if we've already judged this signal
    if signal_id in processed_signals:
        print(f"#{i} DUPLICATE: {signal_id}")
        continue
    
    # Determine if this is worth judging
    worth_judging = (
        score >= 4 and  # Higher confidence
        len(narratives) >= 2 or  # Multi-narrative
        'rug_warning' in narratives or  # Rug warnings are high-value
        len(cashtags) >= 2  # Multiple tokens mentioned
    )
    
    if worth_judging:
        print(f"#{i} JUDGE: Score={score}, Author=@{author}")
        print(f"   Narratives: {narratives}")
        print(f"   Tokens: {cashtags}")
        print(f"   Text: {text}")
        print()
        
        # Mark as processed
        processed_signals.add(signal_id)
        
        # Here we would call cynic_judge:
        # cynic_judge(content=text, domain='token-analysis', ...)
        
    else:
        print(f"#{i} OBSERVE: Score={score}, Author=@{author}, Narratives={narratives}")
        print(f"   Text: {text}")
        print()
        
        # Here we would call cynic_observe:
        # cynic_observe(tool='observe', domain='token-analysis', context=...)

print(f"\nTotal unique signals processed: {len(processed_signals)}")
print(f"Signals ready for judgment: {sum(1 for t in high_signal if t.get('signal_score', 0) >= 4)}")
