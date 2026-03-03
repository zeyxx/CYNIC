# Trader Ralph Integration with CYNIC — Complete TODO List

**Status:** Design phase (not yet started)
**Created:** 2026-03-03
**Owner:** TBD
**Estimated Effort:** 4-6 weeks (full implementation)

---

## 1. DISCOVERY & DESIGN PHASE

### 1.1 Understand Trader Ralph in Detail
- [ ] Document all available endpoints from Trader Ralph API
  - Market data endpoints (OHLCV, quotes, balances)
  - Solana-native signals (marks, scores, top views)
  - Macro signals (verdicts, FRED, ETF flows, stablecoin health)
  - Derivatives data (funding rates, open interest, venue scoring)
  - Exact rate limits and cost model (402 payment)
- [ ] Identify required authentication (wallet, API key, x402 headers)
- [ ] Map data latency requirements (real-time vs hourly vs daily)
- [ ] Determine which signals are most valuable for pattern detection

### 1.2 Assess CYNIC's Current Architecture
- [ ] Verify all adapter patterns exist and are working
  - LLMAdapter pattern (request/response/metadata)
  - MarketSensor polling loop
  - SelfProber proposal generation
  - ProposalExecutor risk classification
  - Q-Table learning mechanism
  - EventBus event routing
- [ ] Document missing components:
  - Data adapter pattern for market feeds (vs LLM adapters)
  - Backtesting harness/replay engine
  - Proposal generation pipeline (signals → proposals)
  - Reporting/dashboard layer
- [ ] Identify existing test infrastructure for integrations

### 1.3 Design Decision Matrix (CHOOSE ONE PATH)

#### Path A: Full Integration (All Signals)
- **Pros:** Maximum pattern surface, learn from macro + micro signals, compound advantage
- **Cons:** More complex backtesting, higher Trader Ralph costs, more failure modes
- **Start with:** Technicals + sentiment, add macro later

#### Path B: Focused Integration (Technicals Only)
- **Pros:** Simpler MVP, fewer signals = easier pattern validation, lower cost
- **Cons:** Limited learning surface, misses macro regime shifts
- **Start with:** SMA/EMA/RSI on 15 pairs

#### Path C: Macro-First (Sentiment + Fear/Greed)
- **Pros:** Detect regime shifts before price moves, fewer false signals
- **Cons:** Delayed execution (sentiment lags price), limited granularity
- **Start with:** BUY/NEUTRAL verdicts + fear/greed index

**Decision Needed:** Which path? Or hybrid?

---

## 2. DATA ADAPTER LAYER

### 2.1 Create MarketDataAdapter Pattern (Template)
- [ ] Design abstract base class: `MarketDataAdapter`
  - Unified request format (symbol, timeframe, indicator_type)
  - Unified response format (price, indicator_value, timestamp, confidence)
  - Error handling + fallback pattern
  - Cost tracking (402 payments)
- [ ] Model after existing LLMAdapter pattern in `cynic/kernel/organism/brain/llm/adapter.py`

### 2.2 Implement TraderRalphAdapter
**File:** `cynic/kernel/organism/perception/adapters/trader_ralph_adapter.py`
- [ ] Client initialization
  - Solana wallet integration (sign 402 requests)
  - API key management
  - Retry logic with exponential backoff
- [ ] Endpoint methods
  - `fetch_ohlcv(symbol, lookback_hours)` → list[OHLCV]
  - `fetch_indicators(symbol)` → {SMA20, EMA20, RSI14, MACD}
  - `fetch_verdict(symbol)` → BUY | NEUTRAL
  - `fetch_funding_rates(venue)` → {hyperliquid_rate, dydx_rate}
  - `fetch_fear_greed()` → float [0, 100]
- [ ] Caching layer (1-hour TTL for OHLCV, 5-min for real-time quotes)
- [ ] Metrics emission (cost per request, latency, success rate)

### 2.3 Create MarketDataRegistry (Like LLMRegistry)
**File:** `cynic/kernel/organism/perception/adapters/market_registry.py`
- [ ] Registry pattern (similar to LLMRegistry)
  - `register(provider_name, adapter)`
  - `get_for(symbol, signal_type)` → adapter
  - `get_available()` → list of adapters
- [ ] Supports multiple providers (Trader Ralph, Binance, etc.)
- [ ] Graceful fallback if primary fails

### 2.4 Wire Adapters into MarketSensor
**File:** `cynic/kernel/organism/perception/senses/market.py`
- [ ] Update `perceive_market()` to use real adapters instead of simulation
- [ ] Emit typed signals to EventBus:
  - `MARKET_SIGNAL_PRICE` (symbol, price, timestamp)
  - `MARKET_SIGNAL_INDICATOR` (symbol, indicator_name, value)
  - `MARKET_SIGNAL_SENTIMENT` (verdict, fear_greed, source)
  - `MARKET_SIGNAL_DERIVATIVE` (funding_rate, venue, symbol)
- [ ] Error handling (adapter failure → emit MARKET_SIGNAL_ERROR)

---

## 3. PROPOSAL PIPELINE

### 3.1 Design Signal → Proposal Transformation
**Question:** How do market signals trigger trading proposals?

#### Option A: Rule-Based
```
IF RSI < 30 AND SMA Bullish AND Fear/Greed > 70 THEN "STRONG BUY" proposal
```
- Simple, deterministic, easy to backtest
- Limited adaptability

#### Option B: Pattern Recognition
```
SelfProber listens to market signals
Detects pattern clusters (e.g., "3x RSI oversold + fear spike + funding drop")
Generates "TRADE" proposal with confidence
```
- Adaptive, learns from outcomes
- More complex validation

#### Option C: Hybrid
Start with rules, migrate to pattern learning as crystallized patterns accumulate

**Decision Needed:** Which approach?

### 3.2 Implement TradingProposalGenerator
**File:** `cynic/kernel/organism/brain/cognition/cortex/trading_proposer.py`
- [ ] Subscribe to market signals (MARKET_SIGNAL_*)
- [ ] Generate proposals with:
  - Symbol (SOL, RAY, WIF, etc.)
  - Action (BUY, SELL, HOLD)
  - Confidence (0-1, based on signal strength)
  - Entry price, target, stop-loss
  - Position size recommendation
  - Rationale (which signals triggered)
- [ ] Emit `TRADING_PROPOSAL_GENERATED` event

### 3.3 Integrate with SelfProber
- [ ] Extend SelfProber to generate trading proposals from market patterns
  - Reuse `SelfProposal` dataclass or create `TradingProposal`
  - Risk dimension: TRADING_RULE | TRADING_PATTERN | MACRO_SIGNAL
- [ ] Proposal persistence (append to ~/.cynic/trading_proposals.json)

---

## 4. BACKTESTING ENGINE

### 4.1 Design Historical Replay System
**Question:** How to validate trading patterns?

#### Approach A: Direct Replay
```
FOR each day in history:
  FOR each signal in Trader Ralph historical data:
    Generate proposal
    Simulate execution at actual price
    Track P&L
  Measure win_rate, sharpe_ratio, max_drawdown
```
- Realistic, uses actual prices
- Requires historical data download

#### Approach B: Monte Carlo
```
Generate 1000 price paths from historical volatility
Run trading signals on each path
Measure distribution of outcomes
```
- Faster, explores uncertainty
- Less realistic

**Decision Needed:** Which approach?

### 4.2 Implement BacktestEngine
**File:** `cynic/kernel/organism/brain/cognition/cortex/backtest_engine.py`
- [ ] Load historical OHLCV from Trader Ralph (or cached file)
- [ ] Replay price feed minute-by-minute (or hour-by-hour)
- [ ] For each timestamp:
  - Fetch historical indicator values
  - Generate trading proposal
  - Simulate order execution (entry, exit)
  - Track position P&L, fees, slippage
- [ ] Output statistics:
  - Win rate (profitable trades / total trades)
  - Avg profit per trade
  - Max drawdown
  - Sharpe ratio
  - Calmar ratio
  - Total trades
- [ ] Support date range filtering

### 4.3 Create BacktestResult Dataclass
```python
@dataclass
class BacktestResult:
    symbol: str
    start_date: datetime
    end_date: datetime
    total_trades: int
    winning_trades: int
    losing_trades: int
    win_rate: float
    avg_profit: float
    max_drawdown: float
    sharpe_ratio: float
    max_consecutive_wins: int
    max_consecutive_losses: int
```

### 4.4 Integration with Proposal Validation
- [ ] Before crystallizing a pattern, run backtest
  - If win_rate > 55% AND sharpe_ratio > 1.0 → can crystallize
  - If win_rate < 40% → forget pattern
- [ ] Emit `BACKTEST_COMPLETED` event with results
- [ ] Store backtests in EventJournal for audit trail

---

## 5. PATTERN CRYSTALLIZATION (CCM Integration)

### 5.1 Map Trading Patterns to CCM
- [ ] Identify pattern metrics that feed CCM stability calculation:
  - `reputation_score` = backtest win_rate
  - `consensus` = frequency of signal occurrence
  - `decay` = time since last profitable trade
- [ ] Stability threshold: Must exceed φ^-1 (0.618) to crystallize

### 5.2 Implement TradingPatternCrystallizer
**File:** `cynic/kernel/organism/brain/cognition/cortex/pattern_crystallizer.py`
- [ ] Subscribe to BACKTEST_COMPLETED events
- [ ] For each backtested pattern:
  - Calculate stability = min(win_rate × φ, consensus, 1 - decay)
  - If stability > 0.618 → crystallize to memory
  - If stability < 0.382 → forget pattern
- [ ] Emit `PATTERN_CRYSTALLIZED` or `PATTERN_FORGOTTEN` event

### 5.3 Create CrystallizedPattern Storage
**File:** `cynic/kernel/core/storage/crystallized_patterns.py`
- [ ] Persist crystallized patterns to SurrealDB table: `crystallized_trading_patterns`
  - pattern_id, signal_rule, metrics (win_rate, sharpe, etc.), crystallized_at, last_used
- [ ] Query interface:
  - `get_crystallized_patterns()` → list of active patterns
  - `get_pattern_by_id(id)` → full pattern details
  - `get_patterns_for_symbol(symbol)` → patterns applicable to symbol

---

## 6. EXECUTION & GUARDRAILS

### 6.1 Define Risk Parameters
- [ ] **Max position size** per trade (e.g., 5% of portfolio)
- [ ] **Max daily loss** (e.g., -2% stop out)
- [ ] **Max leverage** (if using perps, e.g., 3x max)
- [ ] **Max slippage tolerance** (e.g., 0.5%)
- [ ] **Max open positions** at once (e.g., 3)
- [ ] **Min pattern confidence** to execute (e.g., 0.7)

### 6.2 Extend ProposalExecutor for Trading
- [ ] Update risk classification:
  - LOW_RISK: Crystallized pattern (>0.618 stability) + within guardrails
  - REVIEW_REQUIRED: High confidence but new pattern, or large position
  - NOT_EXECUTABLE: Violates guardrails, contradicts CCM beliefs
- [ ] Add execution handlers:
  - `handle_trading_proposal()` → submit order to Jupiter
  - Track position in memory (entry price, size, stop-loss, target)

### 6.3 Settlement Layer
**Decision Needed:** Real Solana or paper trading?

#### Real Settlement
- [ ] Integrate Jupiter SDK for actual swaps
- [ ] Wallet integration (sign transactions)
- [ ] Track actual positions and P&L
- [ ] Handle slippage and execution failures

#### Paper Trading
- [ ] Simulate order fills at market prices
- [ ] Track positions in memory
- [ ] Measure hypothetical P&L
- [ ] No real Solana risk

**Recommendation:** Start with paper trading, graduate to real once patterns validate

---

## 7. MONITORING & REPORTING

### 7.1 Create Trading Dashboard
**File:** `cynic/interfaces/api/routes/trading_dashboard.py`
- [ ] `/api/trading/portfolio` → current positions, unrealized P&L
- [ ] `/api/trading/trades` → trade history with entry/exit/P&L
- [ ] `/api/trading/patterns` → active crystallized patterns + performance
- [ ] `/api/trading/backtest/{pattern_id}` → historical validation
- [ ] `/api/trading/metrics` → win rate, sharpe, drawdown, etc.

### 7.2 Create CLI Commands
**File:** `bin/cynic-trading` entry point
```bash
cynic-trading list-patterns          # Show all crystallized patterns
cynic-trading show {pattern_id}      # Pattern details + backtest results
cynic-trading portfolio              # Current positions + P&L
cynic-trading history [--limit 50]   # Recent trades
cynic-trading backtest {pattern_id}  # Run backtest on pattern
cynic-trading enable {pattern_id}    # Activate pattern for live trading
cynic-trading disable {pattern_id}   # Pause pattern
```

### 7.3 Real-Time Alerts
- [ ] Alert on large open losses (e.g., -1% realized)
- [ ] Alert on pattern crystallization
- [ ] Alert on pattern degradation (win rate falling below threshold)
- [ ] Alert on guardrail violations (tried to trade when daily loss limit hit)

---

## 8. TESTING STRATEGY

### 8.1 Unit Tests
- [ ] TraderRalphAdapter: API calls, response parsing, error handling
- [ ] MarketDataRegistry: adapter registration and fallback
- [ ] TradingProposalGenerator: signal → proposal transformation
- [ ] BacktestEngine: historical replay, metrics calculation
- [ ] ProposalExecutor extensions: trading risk classification

### 8.2 Integration Tests
- [ ] MarketSensor + TraderRalphAdapter: real signal flow
- [ ] TradingProposalGenerator + SelfProber: proposal generation pipeline
- [ ] BacktestEngine + CCM: pattern validation and crystallization
- [ ] ProposalExecutor + settlement: order execution (paper trading)
- [ ] End-to-end: signal → proposal → backtest → crystallize → execute

### 8.3 Backtesting Validation
- [ ] Historical data accuracy (prices match Trader Ralph)
- [ ] Order fill simulation (realistic slippage/spreads)
- [ ] Fee calculation (Jupiter swap fees)
- [ ] P&L accuracy (per-trade and cumulative)

### 8.4 Paper Trading Validation
- [ ] Signals generate at expected frequency
- [ ] Proposals respect guardrails
- [ ] Executed trades logged to EventJournal
- [ ] Dashboard shows accurate positions and P&L

---

## 9. DOCUMENTATION

### 9.1 Architecture Document
- [ ] Write `docs/TRADER_RALPH_INTEGRATION.md`
  - System overview (adapters → proposals → backtesting → crystallization → execution)
  - Signal types and their meanings
  - Proposal generation logic
  - Backtesting methodology
  - CCM integration (stability calculation)
  - Risk guardrails

### 9.2 API Reference
- [ ] Document all new endpoints
- [ ] CLI command usage
- [ ] Data formats (signal, proposal, backtest result, pattern)

### 9.3 Deployment Guide
- [ ] Trader Ralph credentials setup
- [ ] Configuration (risk parameters, signal weights)
- [ ] Paper vs real trading mode
- [ ] Monitoring setup

---

## 10. DEPLOYMENT PHASES

### Phase 1: MVP (Week 1-2)
- [ ] TraderRalphAdapter (1 signal type: price only)
- [ ] MarketSensor integration (real data feed)
- [ ] Basic proposal generation (rule-based: RSI oversold)
- [ ] Manual validation (no automation)
- [ ] Paper trading mode
- **Output:** Real market data flowing into CYNIC, manual proposal review

### Phase 2: Backtesting (Week 2-3)
- [ ] BacktestEngine implementation
- [ ] Multi-signal support (RSI, SMA, sentiment)
- [ ] Pattern validation (win_rate > 55%)
- [ ] Backtest reporting CLI
- **Output:** Ability to validate trading patterns historically

### Phase 3: Crystallization (Week 3-4)
- [ ] CCM integration
- [ ] Pattern crystallization logic
- [ ] Storage of crystallized patterns
- [ ] Automated proposal generation from patterns
- **Output:** Patterns that pass validation auto-generate proposals

### Phase 4: Execution (Week 4-5)
- [ ] ProposalExecutor trading integration
- [ ] Guardrails enforcement
- [ ] Paper order settlement
- [ ] Position tracking + P&L calculation
- **Output:** Auto-executed paper trades with realistic P&L

### Phase 5: Monitoring & Polish (Week 5-6)
- [ ] Dashboard and CLI commands
- [ ] Real-time alerts
- [ ] Comprehensive logging
- [ ] Documentation
- [ ] Real Solana integration (optional, only if paper trading validates)
- **Output:** Production-ready autonomous trading system

---

## 11. SUCCESS CRITERIA

- [ ] **Data flow:** Real Trader Ralph signals reach EventBus < 1 second latency
- [ ] **Proposal generation:** Signals → proposals < 100ms
- [ ] **Backtesting:** Can validate any pattern in < 5 seconds (100 days of data)
- [ ] **Pattern crystallization:** Stable patterns (>0.618 stability) persist in SurrealDB
- [ ] **Execution:** Paper trades executed correctly with realistic P&L
- [ ] **Guardrails:** All risk limits enforced (no violations logged)
- [ ] **Monitoring:** Dashboard shows live portfolio state accurately
- [ ] **Testing:** > 80% code coverage, 100 integration tests passing

---

## 12. OPEN DECISIONS (Waiting on User)

| Decision | Options | Impact | Status |
|----------|---------|--------|--------|
| Signal scope | All signals vs Technicals only vs Macro-first | MVP complexity, learning speed | **PENDING** |
| Backtesting approach | Historical replay vs Monte Carlo | Validation accuracy, speed | **PENDING** |
| Proposal generation | Rule-based vs Pattern learning vs Hybrid | Adaptability, complexity | **PENDING** |
| Settlement mode | Paper trading first vs Real immediately | Risk, validation speed | **PENDING** |
| Success metrics | Win rate threshold, Sharpe ratio, other | Pattern crystallization bar | **PENDING** |
| Execution timing | Real-time vs Hourly vs Daily | Liquidity, slippage, costs | **PENDING** |

---

## 12. RESOURCES NEEDED

- **Trader Ralph API credentials** (wallet, x402 setup)
- **Historical market data** (OHLCV download for 1-2 years)
- **SurrealDB schema** extensions (crystallized_patterns table)
- **Test data** (known trading signals with known outcomes for validation)
- **Reference implementation** (existing trading system to validate against, optional)

---

**Next Steps:** Choose decisions in Section 12, then proceed with Phase 1 implementation.
