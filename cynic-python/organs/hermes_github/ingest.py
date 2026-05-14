#!/usr/bin/env python3
import json
import sys
from pathlib import Path
from datetime import datetime

# Data from initial fetch
STARS = [
    {"name": "VIRATTT / DEXTER", "description": "An autonomous agent for deep financial research", "language": "TypeScript"},
    {"name": "CLOAKHQ / CLOAKBROWSER", "description": "Stealth Chromium that passes every bot detection test. Drop-in Playwright replacement with source-level fingerprint patches.", "language": "Python"},
    {"name": "HUHUSMANG / AWESOME-LLMS-FOR-VULNERABILITY-DETECTION", "description": "Awesome Large Language Models for Vulnerability Detection", "language": "Python"},
    {"name": "SOLANA-FOUNDATION / SOLANA-DEVELOPER-PLATFORM", "description": "Solana Developer Platform API and Dashboard", "language": "TypeScript"},
    {"name": "GITHUB / SPEC-KIT", "description": "Toolkit to help you get started with Spec-Driven Development", "language": "Python"},
    {"name": "PMXT-DEV / PMXT", "description": "CCXT for prediction markets. PMXT is a unified API for trading on Polymarket, Kalshi, and more.", "language": "TypeScript"},
    {"name": "POLYBENCH / POLYBENCH", "description": "The first large-scale, contamination-proof benchmark that evaluates LLMs as autonomous trading agents.", "language": "Python"},
    {"name": "JKORF / POLYMARKET.NET", "description": "A C# client library for the Polymarket REST and Websocket API focusing on clear usage and models", "language": "C#"},
    {"name": "ENT0N29 / POLYBOT", "description": "reverse-engineer every polymarket strategy and trade fast", "language": "Java"},
    {"name": "COSMICSTACK-LABS / MERCURY-AGENT", "description": "Soul-driven AI agent with permission-hardened tools, token budgets, and multi-channel access.", "language": "TypeScript"},
    {"name": "FIREDANCER-IO / FIREDANCER", "description": "Firedancer is Jump Crypto's Solana validator software.", "language": "C"},
    {"name": "AEYAKOVENKO / PERCOLATOR-CLI", "description": "", "language": "TypeScript"},
    {"name": "BUG0INC / PASSMARK", "description": "Open-source Playwright library for AI browser regression testing with intelligent caching and auto-healing.", "language": "TypeScript"},
    {"name": "SURREALDB / SURREALDB", "description": "A scalable, distributed, collaborative, document-graph database, for the realtime web", "language": "Rust"},
    {"name": "ABOVESPEC / LOCAL-LLM-BENCHMARKS", "description": "Benchmarks, guides, and results for running LLMs locally on consumer GPUs", "language": ""},
    {"name": "TBOULET / ALAN-CODE-AGENT", "description": "Open-source Python implementation of a Claude-Code-like agent (Gemini CLI, Codex, Copilot...).", "language": "Python"},
    {"name": "ZAKIRKUN / DEEP-EYE", "description": "AI-driven vulnerability scanner and penetration testing tool integrating multiple AI providers.", "language": "Python"},
    {"name": "CCXT / CCXT", "description": "A cryptocurrency trading API with more than 100 exchanges in JS / TS / Python / C# / PHP / Go", "language": "Python"},
    {"name": "0XNYK / AWESOME-HERMES-AGENT", "description": "A curated list of awesome skills, tools, integrations, and resources for Hermes Agent", "language": ""},
    {"name": "KAMINOCORP / HERMES-ALPHA", "description": "Cloud deployed version of the Nous Research Hermes agent", "language": "Python"},
    {"name": "JOEYNYC / HERMES-HUD", "description": "TUI consciousness monitor for Hermes", "language": "Python"},
    {"name": "NATIV3AI / HERMES-AGENT-CAMEL", "description": "Hermes Agent fork with integrated CaMeL trust boundaries", "language": "Python"},
    {"name": "REINAMACCREDY / MAESTRO", "description": "Conductor for cross-agent coding workflows: structured memory, handoffs, and coordination.", "language": "TypeScript"},
    {"name": "ROMANESCU11 / HERMES-SKILL-FACTORY", "description": "A meta-skill plugin for Hermes AI agent that automatically turns workflows into reusable skills.", "language": "Python"},
    {"name": "XASPX / HERMES-CONTROL-INTERFACE", "description": "A self-hosted web dashboard for the Hermes AI agent stack.", "language": "JavaScript"},
    {"name": "KSIMBACK / HERMES-ECOSYSTEM", "description": "\u2312 Hermes Atlas \u2014 the community map of every tool, skill, and integration for Hermes Agent.", "language": "HTML"},
    {"name": "CCLANK / HERMES-WIKI", "description": "Hermes agent + LLM Wiki + \u6e90\u4ee3\u7801 \u5b8c\u6210 Hermes agent wiki", "language": ""},
    {"name": "D4VINCI / SCRAPLING", "description": "\ud83d\udd77\ufe0f An adaptive Web Scraping framework that handles everything from a single request to a full-scale crawl!", "language": "Python"},
    {"name": "SAMUELTUYIZERE / OC-GO-CC", "description": "Use your OpenCode Go subscription with Claude Code.", "language": "Go"},
    {"name": "NOONGHUNNA / CLUB-3090", "description": "Community recipes for serving LLMs on RTX 3090.", "language": "Python"},
]

def ingest():
    output_dir = Path.home() / ".cynic" / "organs" / "hermes" / "github"
    output_dir.mkdir(parents=True, exist_ok=True)
    output_file = output_dir / "stars.jsonl"
    
    with open(output_file, 'w') as f:
        for star in STARS:
            star['ingested_at'] = datetime.now().isoformat()
            f.write(json.dumps(star) + '\n')
    
    print(f"Ingested {len(STARS)} stars to {output_file}")

if __name__ == "__main__":
    ingest()
