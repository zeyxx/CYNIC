/**
 * CYNIC LLM Adapters
 *
 * Provider-specific adapters for LLM integration.
 *
 * @module @cynic/llm/adapters
 */

'use strict';

// Base adapter
export { LLMAdapter } from './base.js';

// Claude Code adapter (pass-through for Claude Code environment)
export { ClaudeCodeAdapter } from './claude-code.js';

// OSS LLM adapters (Ollama, OpenAI-compatible)
export {
  OSSLLMAdapter,
  createOllamaValidator,
  createLMStudioValidator,
  createOpenAIValidator,
} from './oss-llm.js';

// AirLLM adapter (large models via disk offloading)
export {
  AirLLMAdapter,
  createAirLLMValidator,
  checkAirLLMAvailability,
} from './airllm.js';

// Gemini adapter (Google Generative AI)
export {
  GeminiAdapter,
  createGeminiValidator,
} from './gemini.js';

// Re-export types for convenience
export { LLMResponse, ConsensusResult } from '../types.js';
