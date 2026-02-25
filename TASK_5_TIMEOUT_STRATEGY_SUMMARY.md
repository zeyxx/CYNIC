# TASK 5: Context-Aware Timeout Strategy Implementation

**Status**: COMPLETE ✓ | Commit: `4067a3b`

## Overview

Implemented context-aware timeout strategy for MCP tool calls to prevent spurious timeouts on long-running batch operations while maintaining responsiveness for quick operations.

**Key Problem Solved**: Batch jobs (empirical tests, axiom testing) can take 5+ minutes but were timing out under generic 30-second timeout.

## Architecture

### TimeoutCategory (Enum)

Four categories optimized for different operation types:

```
FAST   = 2.0s      (health checks, status queries)
NORMAL = 30.0s     (ask, observe, learn operations)
BATCH  = 300.0s    (empirical tests - 5 minute jobs)
STREAM = ∞         (indefinite - watch_telemetry, watch_source)
```

### TimeoutConfig (Central Registry)

Maintains centralized mapping of 17 tools to timeout categories:

**FAST Tools (2s)**:
- cynic_health
- cynic_status
- cynic_get_job_status
- cynic_get_kernel_status
- cynic_ping

**NORMAL Tools (30s)**:
- ask_cynic
- observe_cynic
- learn_cynic
- discuss_cynic
- cynic_query_telemetry
- cynic_get_axioms
- cynic_get_dogs
- cynic_get_q_table

**BATCH Tools (300s)**:
- cynic_run_empirical_test
- cynic_test_axiom_irreducibility
- cynic_benchmark_learning_efficiency
- cynic_run_load_test

**STREAM Tools (∞)**:
- cynic_watch_telemetry
- cynic_watch_source
- cynic_stream_judgments

### Adapter Integration

Updated `ClaudeCodeAdapter` with two new methods:

1. **`_get_timeout_for_tool(tool_name)`**
   - Looks up tool in TimeoutConfig
   - Returns timeout in seconds or None
   - Defaults to NORMAL (30s) if unknown

2. **`_call_with_timeout(tool_name, coro)`**
   - Wraps coroutine with `asyncio.wait_for()`
   - Applies context-aware timeout
   - Logs timeout violations
   - Stream tools (None timeout) execute indefinitely

### Updated Methods

All major tool call methods now use context-aware timeouts:

| Method | Category | Timeout |
|--------|----------|---------|
| is_cynic_ready | FAST | 2s |
| ask_cynic | NORMAL | 30s |
| teach_cynic | NORMAL | 30s |
| start_empirical_test | BATCH | 300s |
| poll_test_progress | BATCH | 300s |
| get_test_results | NORMAL | 30s |
| test_axiom_irreducibility | BATCH | 300s |
| query_telemetry | NORMAL | 30s |
| stream_telemetry | STREAM | ∞ |

## Test Coverage

### Test File: `cynic/tests/mcp/test_timeout_strategy.py`

**27 comprehensive tests** organized into 5 test classes:

#### TestTimeoutCategory (5 tests)
- ✓ FAST timeout value = 2.0
- ✓ NORMAL timeout value = 30.0
- ✓ BATCH timeout value = 300.0
- ✓ STREAM timeout value = None
- ✓ Float conversion works correctly

#### TestTimeoutConfig (9 tests)
- ✓ FAST tools have 2s timeout
- ✓ NORMAL tools have 30s timeout
- ✓ BATCH tools have 300s timeout
- ✓ STREAM tools have no timeout
- ✓ get_category() returns correct enum
- ✓ Unknown tools default to NORMAL
- ✓ summary() returns correct structure
- ✓ Tools in summary are alphabetically sorted
- ✓ All configured tools appear in summary

#### TestTimeoutApplication (5 tests)
- ✓ FAST tool timeout enforced after 2s
- ✓ NORMAL tool timeout enforced after 30s
- ✓ STREAM tools have no timeout
- ✓ Operations within timeout succeed
- ✓ _get_timeout_for_tool() helper works

#### TestTimeoutEdgeCases (3 tests)
- ✓ Registry includes all essential tools
- ✓ Timeout values are reasonable and ordered
- ✓ Safe operations under timeout work

#### TestTimeoutDocumentation (5 tests)
- ✓ TimeoutCategory has docstring
- ✓ TimeoutConfig has docstring
- ✓ get_timeout() has docstring
- ✓ get_category() has docstring
- ✓ summary() has docstring

### Test Results

```
74 passed, 3 skipped, 1 warning in 42.51s

✓ 27 new timeout strategy tests
✓ 47 existing MCP tests (still passing)
✓ No regressions
```

## Code Quality

### Documentation
- TimeoutCategory: Enum with docstrings
- TimeoutConfig: Class with docstrings and type hints
- Adapter methods: Updated docstrings showing timeout category
- Code comments: Clear explanations in each section

### Type Safety
- Type hints on all public methods
- Return types clearly specified
- Optional[float] for timeouts (None = indefinite)

### Error Handling
- Graceful asyncio.TimeoutError handling
- Logging of timeout violations
- Fallback to NORMAL (30s) for unknown tools

## Usage Examples

### Basic Usage

```python
from cynic.mcp.claude_code_adapter import ClaudeCodeAdapter

adapter = ClaudeCodeAdapter()

# Ask question (30s timeout - NORMAL)
result = await adapter.ask_cynic("Is this code secure?")

# Run long empirical test (300s timeout - BATCH)
test_result = await adapter.start_empirical_test(count=10000)

# Watch telemetry (indefinite - STREAM)
summary = await adapter.stream_telemetry(duration_s=60)
```

### Getting Timeout Information

```python
from cynic.mcp.timeouts import TimeoutConfig

# Get timeout for a tool
timeout = TimeoutConfig.get_timeout("ask_cynic")  # 30.0

# Get category
category = TimeoutConfig.get_category("cynic_run_empirical_test")  # BATCH

# Get full summary
summary = TimeoutConfig.summary()
# {
#   "FAST": {"timeout_s": 2.0, "tools": [...]},
#   "NORMAL": {"timeout_s": 30.0, "tools": [...]},
#   ...
# }
```

## Files Changed

### Created
- `cynic/mcp/timeouts.py` (107 LOC)
  - TimeoutCategory enum
  - TimeoutConfig class
  - Registry of 17 tools

- `cynic/tests/mcp/test_timeout_strategy.py` (331 LOC)
  - 27 comprehensive tests
  - 5 test classes covering all scenarios

### Modified
- `cynic/mcp/claude_code_adapter.py` (+156 LOC, ~100 modified)
  - Added imports for TimeoutConfig
  - Added _get_timeout_for_tool() helper
  - Added _call_with_timeout() wrapper
  - Updated all 9 tool call methods

## Benefits

### 1. Prevents Spurious Timeouts
- Batch jobs (300s) won't timeout under 30s limit
- Empirical tests can run for full duration
- Axiom testing completes successfully

### 2. Maintains Responsiveness
- Health checks fast-fail at 2s
- Status queries don't wait unnecessarily
- Quick operations respond quickly

### 3. Centralized Policy
- Single source of truth (TOOL_TIMEOUTS dict)
- Easy to adjust timeouts
- No timeout magic numbers scattered in code

### 4. Type-Safe
- Enum ensures valid timeout values
- Type hints on all methods
- Mypy-friendly implementation

### 5. Well-Tested
- 27 tests covering all scenarios
- Edge cases handled
- No regressions

## Performance Impact

- Minimal: TimeoutConfig is just a dict lookup
- No network overhead: Timeouts are local asyncio operations
- Actual timeout enforcement via asyncio.wait_for() (standard library)

## Future Enhancements

1. **Dynamic Timeout Adjustment**
   - Could adjust batch timeout based on empirical test size
   - Could learn optimal timeouts from historical data

2. **Timeout Hooks**
   - Could notify user when approaching timeout
   - Could implement graceful cleanup on timeout

3. **Timeout Metrics**
   - Could track timeout frequency per tool
   - Could detect tools that need timeout adjustment

4. **Streaming Timeout Option**
   - Currently STREAM = indefinite
   - Could add configurable stream duration

## Deployment Notes

- No database changes
- No API changes (timeouts are internal)
- Backward compatible (NORMAL default for unknown tools)
- No breaking changes to existing code

## Related Documentation

- `.claude/KERNEL_GUIDANCE.md` — Timeout philosophy
- `cynic/mcp/timeouts.py` — Implementation details
- `cynic/tests/mcp/test_timeout_strategy.py` — Test coverage

## Conclusion

Task 5 successfully implements context-aware timeout strategy, solving the critical problem of spurious timeouts on batch operations while maintaining responsiveness for quick operations. All 27 tests pass, with no regressions to existing functionality.

Ready for production use. Prevents timeout frustration on long-running jobs while keeping health checks snappy.

---

**Confidence**: 61.8% (φ⁻¹) — Implementation complete, tested, and deployable.
