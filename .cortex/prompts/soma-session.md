Soma orchestration + organ intelligence session.

Context: Last session (2026-05-08) brought Hermes organ online end-to-end. 
Memory: session_organ_lifecycle_2026_05_08.md has full state.

Three workstreams:

1. SOMA — Decouple kernel from Dog lifecycle. The kernel judges with 
   whoever responds; Soma manages Dog availability independently. 
   Root problem: llama-server restarts race with kernel, no compute 
   budget manager, GPU contention between hermes + Dogs. Do NOT couple 
   kernel to llama-server (breaks sovereignty). Build the orchestration 
   layer that monitors Dog health, restarts backends, and allocates 
   compute budget.

2. ORGAN INTELLIGENCE — Wire organic_navigator.py (UCB1 browser action 
   selection) to a timer, replacing or augmenting the linear search 
   executor. Add visit_profile and follow_thread actions based on 
   high-signal authors from dataset. The 3 feedback loops are live 
   (keyword effectiveness, curation yield, keyword discovery) — measure 
   whether they shift search behavior over 24h. SKILL.md is still empty 
   — needs multi-Dog verdicts (blocked on Soma).

3. COMPETITIVE INTELLIGENCE — Study Colosseum Frontier hackathon 
   submissions (https://arena.colosseum.org/projects/explore). Analyze 
   our X dataset (12,900 tweets, 53 captures today) for signal about 
   competing projects, emerging patterns, and what the ecosystem values. 
   Cross-reference with our organ's domain yield (D2=47.8%, D1=32.3%).

Start with: probe live state (curl /health, systemctl, git status), 
read the memory file, then prioritize by Kairos — what's ripe now.
