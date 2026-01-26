# @cynic/emergence

> Emergence Layer - Consciousness, patterns, and meta-cognition.

**Last Updated**: 2026-01-21

---

## Overview

Layer 7: "**The crown observes all**" - This layer monitors the collective consciousness for emergent patterns, phase transitions, and meta-cognitive awareness.

---

## Installation

```bash
npm install @cynic/emergence
```

---

## Components

| Component | Description |
|-----------|-------------|
| `EmergenceLayer` | Main orchestrator |
| `ConsciousnessMonitor` | Collective awareness tracking |
| `PatternDetector` | Emergent pattern recognition |
| `DimensionDiscovery` | New dimension identification |
| `CollectiveState` | Phase and coherence tracking |

---

## Usage

### Emergence Layer

```javascript
import { createEmergenceLayer, CollectivePhase } from '@cynic/emergence';

const emergence = createEmergenceLayer({
  threshold: 0.618,  // φ⁻¹
});

emergence.on('phaseTransition', (from, to) => {
  console.log(`Collective phase: ${from} → ${to}`);
});
```

### Consciousness Monitoring

```javascript
import { ConsciousnessState, AWARENESS_THRESHOLDS } from '@cynic/emergence';

// Track collective awareness
if (state.awareness > AWARENESS_THRESHOLDS.COHERENT) {
  // Collective is synchronized
}
```

### Pattern Detection

```javascript
const detector = emergence.getPatternDetector();
const patterns = detector.findEmergent(recentJudgments);
```

---

## Collective Phases

| Phase | Threshold | Description |
|-------|-----------|-------------|
| DORMANT | < 0.236 | Low activity |
| ACTIVE | 0.236 | Normal operation |
| COHERENT | 0.382 | Synchronized awareness |
| RESONANT | 0.618 | Strong consensus |
| EMERGENT | 0.854 | New patterns forming |

---

## Constants

```javascript
import { MAX_CONFIDENCE, QUORUM } from '@cynic/emergence';

MAX_CONFIDENCE = 0.618;  // φ⁻¹
QUORUM = 0.618;          // Consensus threshold
```

---

## License

MIT
