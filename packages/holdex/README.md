# @cynic/holdex

> HolDex integration - K-Score token quality analysis.

**Last Updated**: 2026-01-21

---

## Overview

HolDex provides **K-Score** analysis for Solana tokens, measuring holder quality and distribution patterns.

CYNIC uses K-Score as an input to token judgments.

---

## Installation

```bash
npm install @cynic/holdex
```

---

## Usage

### Basic Query

```javascript
import { HolDexClient } from '@cynic/holdex';

const client = new HolDexClient();

const result = await client.getKScore('So11...');
console.log(result.kScore);      // 0-100
console.log(result.tier);        // S/A/B/C/D/F
console.log(result.breakdown);   // Dimension scores
```

### With CYNIC Harmony

```javascript
import { HolDexHarmony } from '@cynic/holdex';

const harmony = new HolDexHarmony(cynicJudge);

// Combines K-Score with CYNIC's token judgment
const analysis = await harmony.analyzeToken('So11...');
```

---

## K-Score Dimensions

| Dimension | Description |
|-----------|-------------|
| Concentration | Top holder concentration |
| Distribution | Holder count distribution |
| Activity | Trading activity patterns |
| Age | Token age and stability |
| Growth | Holder growth rate |

---

## K-Score Tiers

| Tier | Score Range | Quality |
|------|-------------|---------|
| S | 90-100 | Exceptional |
| A | 75-89 | Strong |
| B | 60-74 | Good |
| C | 40-59 | Average |
| D | 20-39 | Weak |
| F | 0-19 | Poor |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HOLDEX_API_URL` | HolDex API endpoint |
| `HOLDEX_API_KEY` | API key (if required) |

---

## License

MIT
