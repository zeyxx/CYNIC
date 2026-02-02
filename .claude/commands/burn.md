---
name: burn
description: "Vision → Compréhension → Burn. Analyze codebase for simplification opportunities using static analysis + LLM understanding."
arguments:
  - name: mode
    description: "quick (static only) or deep (with LLM)"
    required: false
    default: "quick"
---

# /burn - Codebase Burn Analysis

Vision → Compréhension → Burn

Analyze the codebase for simplification opportunities:
- **Orphans**: Files never imported (candidates for deletion)
- **Hotspots**: Files with too many dependencies (need simplification)
- **Giants**: Files > 500 lines (need splitting)
- **Duplicates**: Similar files (need merging)

## Instructions

1. Load the BurnAnalyzer from `packages/persistence/src/services/burn-analyzer.js`
2. Run analysis based on mode:
   - `quick`: Static analysis only (fast)
   - `deep`: Static + LLM comprehension (thorough)
3. Display results in CYNIC TUI format
4. Highlight top 3 actionable candidates
5. For each candidate, show: path, verdict, reason, confidence

## Output Format

```
═══════════════════════════════════════════════════════════════
🔥 BURN ANALYSIS - "φ distrusts φ"
═══════════════════════════════════════════════════════════════

── CODEBASE STATS ─────────────────────────────────────────────
   Files: X | Lines: X | Packages: X

── ISSUES FOUND ───────────────────────────────────────────────
   🔴 Orphans:    X files (never imported)
   🟠 Hotspots:   X files (>13 dependencies)
   🟡 Giants:     X files (>500 lines)
   🟣 Duplicates: X files (similar structure)

── TOP CANDIDATES ─────────────────────────────────────────────
   1. [VERDICT] path/to/file.js
      Reason: ...
      Confidence: X%

   2. [VERDICT] path/to/file.js
      ...

── SUMMARY ────────────────────────────────────────────────────
   Actionable: X candidates
   Potential lines removable: X

═══════════════════════════════════════════════════════════════
```

## Verdicts

- **DELETE**: Can be removed entirely
- **MERGE**: Should be consolidated with similar files
- **SPLIT**: Too big, should be broken into smaller files
- **SIMPLIFY**: Overly complex, needs refactoring
- **KEEP**: Necessary, leave alone
- **REVIEW**: Needs human judgment

## Mode: $mode

Run the appropriate analysis and display results.
