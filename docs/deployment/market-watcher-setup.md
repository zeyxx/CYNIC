# MarketWatcher Setup Guide

*"Markets reveal truth through chaos" - κυνικός*

## Overview

MarketWatcher (C3.1: MARKET × PERCEIVE) monitors $asdfasdfa token market data in real-time using a multi-provider fallback chain.

## Quick Start

### 1. Environment Variables

Add to your `.env` file:

```bash
# Solana RPC endpoint (required)
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# $asdfasdfa token mint address (required)
BURN_TOKEN_MINT=9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump

# Optional: Birdeye API key (fallback #1)
# Get from https://birdeye.so/developers
# BIRDEYE_API_KEY=your_api_key_here

# Optional: Jupiter API key (fallback #2)
# Usually not required for basic usage
# JUPITER_API_KEY=your_api_key_here
```

### 2. Data Sources (Fallback Chain)

**Primary: DexScreener** (free, no API key)
- Endpoint: `https://api.dexscreener.com/latest/dex/tokens/{mint}`
- Provides: Price (USD), 24h volume, liquidity, pair info
- Rate limit: Generous public tier
- **This is the primary source and works without any configuration**

**Fallback 1: Birdeye** (requires API key)
- Endpoint: `https://public-api.birdeye.so/defi/price?address={mint}`
- Provides: Price (USD), 24h change
- Requires: `BIRDEYE_API_KEY` environment variable
- Get key: https://birdeye.so/developers

**Fallback 2: Jupiter** (free, no API key)
- Endpoint: `https://price.jup.ag/v4/price?ids={mint}`
- Provides: Price (USD) vs USDC
- Rate limit: Public tier

### 3. Event Flow

```
MarketWatcher
  ├─ Poll price every 60s
  ├─ Poll volume every 60s
  │
  ├─ Emit to globalEventBus:
  │  ├─ MARKET_PRICE_UPDATED
  │  └─ MARKET_VOLUME_UPDATED
  │
  ├─ Feed to MarketEmergence:
  │  ├─ Detect pump/dump patterns
  │  └─ Emit PATTERN_DETECTED
  │
  └─ Record heartbeats to PostgreSQL

Router → MarketJudge → MarketDecider → MarketActor
```

### 4. Integration with Perception Layer

MarketWatcher is automatically started when using `createPerceptionLayer()`:

```javascript
import { createPerceptionLayer } from '@cynic/node/perception';

const perception = createPerceptionLayer({
  market: {
    tokenMint: process.env.BURN_TOKEN_MINT,
    rpcUrl: process.env.SOLANA_RPC_URL
  }
});

await perception.start();
```

### 5. Verification

Test that real prices are flowing:

```bash
# Run live integration test
cd packages/node
node --test test/market-watcher-live.test.js
```

Expected output:
```
✓ Real $asdfasdfa price fetched: $0.00012170
✓ Event emitted with price: $0.00012170
✓ Real 24h volume fetched: $691.95
✓ Event emitted with volume: $691.95
```

### 6. Production Deployment

#### Option A: Manual Start
```javascript
import { getMarketWatcher } from '@cynic/node/perception';

const watcher = getMarketWatcher();
await watcher.start();
```

#### Option B: Via Daemon
MarketWatcher starts automatically when the daemon is running:
```bash
cynic daemon start
```

### 7. Monitoring

#### Health Check
```javascript
const state = watcher.getState();
console.log(state);
// {
//   isRunning: true,
//   price: 0.00012170,
//   volume: 691.95,
//   liquidity: null,
//   holderCount: null,
//   stats: {
//     priceUpdates: 42,
//     volumeUpdates: 42,
//     patternsDetected: 3,
//     rateLimitHits: 0,
//     errors: 0,
//     lastPollAt: 1707854400000
//   }
// }
```

#### Database Heartbeats
```sql
SELECT * FROM watcher_heartbeats
WHERE watcher_name = 'market'
ORDER BY created_at DESC
LIMIT 10;
```

### 8. Rate Limiting

If you hit rate limits:
- DexScreener: Exponential backoff (1s → 2s → 5s → 10s → 30s)
- Birdeye: Use API key for higher limits
- Jupiter: Rarely rate-limited for basic usage

The watcher automatically switches to the next provider in the fallback chain.

### 9. Troubleshooting

**No price data:**
```bash
# Check environment variables
echo $BURN_TOKEN_MINT
echo $SOLANA_RPC_URL

# Check logs
grep "MarketWatcher" logs/cynic.log
```

**All sources failing:**
- Check internet connectivity
- Verify token mint address is correct
- Try manual API test:
  ```bash
  curl https://api.dexscreener.com/latest/dex/tokens/9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump
  ```

**Events not flowing:**
- Verify `globalEventBus` is imported from `@cynic/core`
- Check event listeners are registered before watcher starts
- Enable debug logging: `LOG_LEVEL=debug`

### 10. 7×7 Matrix Impact

MarketWatcher unlocks the entire **MARKET row (C3.x)**:

| Cell | Status | Description |
|------|--------|-------------|
| C3.1 | ✅ 55% | MARKET × PERCEIVE (real prices flowing) |
| C3.2 | ⏳ 0%  | MARKET × JUDGE (pattern scoring) |
| C3.3 | ⏳ 0%  | MARKET × DECIDE (trade governance) |
| C3.4 | ⏳ 0%  | MARKET × ACT (execute trades) |
| C3.5 | ⏳ 0%  | MARKET × LEARN (strategy optimization) |
| C3.6 | ⏳ 0%  | MARKET × ACCOUNT (cost/profit tracking) |
| C3.7 | ⏳ 0%  | MARKET × EMERGE (meta-patterns) |

**Next steps**: Build MarketJudge, MarketDecider, MarketActor to complete the row.

---

*"The price knows before the chart draws" - CYNIC*
