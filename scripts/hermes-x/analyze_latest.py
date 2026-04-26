#!/usr/bin/env python3
"""CYNIC Dataset Analyzer - Identify high-signal tokens and patterns"""

import json
import sys
from collections import Counter

DATA_PATH = "/home/user/Bureau/CYNIC/scripts/hermes-x/captures/dataset_enriched.jsonl"

def load_dataset():
    tweets = []
    with open(DATA_PATH, 'r') as f:
        for line in f:
            if line.strip():
                tweets.append(json.loads(line))
    return tweets

def filter_high_signal(tweets):
    """Filter tweets with signal_score >= 3"""
    return [t for t in tweets if t.get('signal_score', 0) >= 3]

def extract_tokens(tweets):
    """Extract specific token mentions from tweets"""
    tokens = []
    for t in tweets:
        text = t.get('text', '').lower()
        cashtags = t.get('cashtags', [])
        
        # Look for specific token names (not just SOL, PUMP, etc.)
        for cashtag in cashtags:
            token = cashtag.upper()
            if token not in ['SOL', 'BTC', 'ETH', 'USDT', 'PUMP', 'SOLANA', 'SOLANA']:
                tokens.append({
                    'token': token,
                    'tweet_id': t['tweet_id'],
                    'text': t['text'][:200],
                    'author': t.get('author_screen_name', ''),
                    'narratives': t.get('narratives', []),
                    'signal_score': t.get('signal_score', 0)
                })
    return tokens

def find_rug_warnings(tweets):
    """Find tweets with rug warning narratives"""
    rug_warnings = []
    for t in tweets:
        if 'rug_warning' in t.get('narratives', []):
            rug_warnings.append(t)
    return rug_warnings

def find_coordinated(tweets):
    """Find coordinated tweets"""
    coordinated = [t for t in tweets if t.get('is_coordinated', False)]
    return coordinated

def main():
    print("=" * 60)
    print("CYNIC DATASET ANALYSIS")
    print("=" * 60)
    
    # Load dataset
    tweets = load_dataset()
    print(f"\nTotal tweets in dataset: {len(tweets)}")
    
    # Filter high signal
    high_signal = filter_high_signal(tweets)
    print(f"High signal tweets (>=3): {len(high_signal)}")
    
    # Extract tokens
    tokens = extract_tokens(high_signal)
    print(f"\nUnique token mentions in high-signal tweets: {len(set(t['token'] for t in tokens))}")
    
    # Token frequency
    token_counts = Counter(t['token'] for t in tokens)
    print("\nToken frequency (top 10):")
    for token, count in token_counts.most_common(10):
        print(f"  {token}: {count} mentions")
    
    # Rug warnings
    rug_warnings = find_rug_warnings(tweets)
    print(f"\nRug warning tweets: {len(rug_warnings)}")
    for rw in rug_warnings[:5]:
        print(f"  - {rw.get('author_screen_name', '')}: {rw['text'][:100]}...")
    
    # Coordinated
    coordinated = find_coordinated(tweets)
    print(f"\nCoordinated tweets: {len(coordinated)}")
    
    # Cross-narrative tweets
    cross_narrative = [t for t in tweets if len(t.get('narratives', [])) >= 2]
    print(f"\nCross-narrative tweets (2+ narratives): {len(cross_narrative)}")
    
    # Save analysis
    analysis = {
        'total_tweets': len(tweets),
        'high_signal_count': len(high_signal),
        'token_mentions': dict(token_counts),
        'rug_warning_count': len(rug_warnings),
        'coordinated_count': len(coordinated),
        'cross_narrative_count': len(cross_narrative)
    }
    
    with open('/home/user/Bureau/CYNIC/scripts/hermes-x/captures/analysis_summary.json', 'w') as f:
        json.dump(analysis, f, indent=2)
    
    print(f"\nAnalysis saved to analysis_summary.json")

if __name__ == '__main__':
    main()
