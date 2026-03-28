# Adaptive Context Routing — Design Document

*Crystallized 2026-03-15. φ-bounded confidence.*

## Problem

The kernel sends the same prompt to ALL Dogs regardless of their context capacity. A 25K-token stimulus sent to a 4K backend silently truncates or errors. No protection, no routing, no transparency.

## Truth Statements

| T# | Truth | Confidence | Impact |
|----|-------|------------|--------|
| T1 | `BackendCapability` must gain `max_context_tokens: u32`. Discovered via `/props` for llama.cpp, configured in `backends.toml` for cloud. Default 0 = unknown/unlimited. | 58% | Add field to backend.rs, config.rs, backend_llamacpp.rs |
| T2 | Token estimation via `(system_prompt.len() + user_prompt.len()) / 4` is sufficient for routing. No tokenizer library needed. Post-hoc `prompt_tokens` from responses calibrate over time via CCM. | 52% | 5-line free function in router module |
| T3 | When no backend has sufficient context, return `BackendError::ContextExceeded { needed, max_available }`. Never truncate silently. Let the caller decide. | 55% | New enum variant in BackendError |
| T4 | The routing filter belongs in `BackendRouter::route()` as a pre-filter before round-robin. Add `estimated_tokens: Option<u32>` to InferenceRequest. ~15 lines. | 50% | Minimal API change, big safety gain |

## Implementation

### Step 1: Config
```toml
# backends.toml
[backend.gemma-sovereign]
base_url = "http://<TAILSCALE_CORE>:8080/v1"
model = "gemma-sovereign"
context_size = 4096    # NEW FIELD

[backend.gemini]
base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
model = "gemini-3-flash-preview"
context_size = 1000000  # Gemini supports 1M
```

### Step 2: Token estimation
```rust
fn estimate_tokens(system: &str, user: &str) -> u32 {
    ((system.len() + user.len()) as f64 / 4.0).ceil() as u32
}
```

### Step 3: Route filter
```rust
// In BackendRouter::route()
let available = backends.iter()
    .filter(|b| b.is_available())
    .filter(|b| match estimated_tokens {
        Some(tokens) => b.capability().max_context_tokens == 0
            || b.capability().max_context_tokens >= tokens,
        None => true,
    });
```

### Step 4: Error
```rust
pub enum BackendError {
    // ...existing...
    ContextExceeded { needed: u32, max_available: u32 },
}
```

## What This Enables
- Short chess moves → all Dogs (fast consensus)
- Long documents → only 32K+ backends (honest about capacity)
- Massive codebases → only cloud APIs (sovereignty tradeoff made explicit)
- The Verdict reports which Dogs participated and why others were skipped

## What This Does NOT Do
- No truncation
- No summarization
- No tokenizer dependency
- No domain-specific preprocessing
