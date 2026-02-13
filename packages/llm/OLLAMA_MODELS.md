# Ollama Model Adapters

CYNIC supports multiple Ollama models via convenient factory functions.

## Supported Models

| Model | Function | Default Version | Provider |
|-------|----------|----------------|----------|
| Llama | `createLlamaValidator()` | `llama3.2` | Meta |
| Mistral | `createMistralValidator()` | `mistral` | Mistral AI |
| DeepSeek | `createDeepSeekValidator()` | `deepseek-coder` | DeepSeek |
| Qwen | `createQwenValidator()` | `qwen2.5` | Alibaba |

## Setup

### 1. Install Ollama

```bash
# macOS/Linux
curl -fsSL https://ollama.com/install.sh | sh

# Windows
# Download from https://ollama.com/download
```

### 2. Pull Models

```bash
# Pull specific models
ollama pull llama3.2
ollama pull mistral
ollama pull deepseek-coder
ollama pull qwen2.5

# Or pull all at once
ollama pull llama3.2 && \
ollama pull mistral && \
ollama pull deepseek-coder && \
ollama pull qwen2.5
```

### 3. Verify Installation

```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Test a model
ollama run llama3.2 "Say hello"
```

## Usage

### Basic Usage

```javascript
import {
  createLlamaValidator,
  createMistralValidator,
  createDeepSeekValidator,
  createQwenValidator,
} from '@cynic/llm';

// Create adapters
const llama = createLlamaValidator();
const mistral = createMistralValidator();
const deepseek = createDeepSeekValidator();
const qwen = createQwenValidator();

// Use an adapter
const response = await llama.complete('Explain phi in simple terms', {
  temperature: 0.7,
  maxTokens: 512,
});

console.log(response.content);
// => "Phi (φ) is the golden ratio..."
```

### Custom Configuration

```javascript
// Override model version
const llama3_1 = createLlamaValidator({
  model: 'llama3.1',
});

// Custom endpoint (remote Ollama instance)
const remoteMistral = createMistralValidator({
  endpoint: 'http://192.168.1.100:11434',
});

// Longer timeout for large models
const deepseek = createDeepSeekValidator({
  timeout: 60000, // 60s
});
```

### With UnifiedLLMRouter

```javascript
import { getUnifiedLLMRouter } from '@cynic/node/orchestration/unified-llm-router';
import { createLlamaValidator, createMistralValidator } from '@cynic/llm';

const router = getUnifiedLLMRouter();

// Adapters are auto-detected if Ollama is running
// Or manually configure:
router.addAdapter(createLlamaValidator());
router.addAdapter(createMistralValidator());
router.addAdapter(createDeepSeekValidator());
router.addAdapter(createQwenValidator());

// Router will use Thompson Sampling to select best model
const response = await router.call('Write a function to check if a number is prime', {
  strategy: 'best', // Thompson Sampling
  complexity: 'moderate',
});
```

## Environment Variables

Configure Ollama endpoint and default models via environment:

```bash
# .env
OLLAMA_ENDPOINT=http://localhost:11434
OLLAMA_MODEL=llama3.2
```

## Model Characteristics

### Llama 3.2 (Meta)
- **Best for**: General reasoning, conversation, code understanding
- **Size**: 3B - 90B parameters
- **Speed**: Fast (small models), moderate (large models)
- **Strengths**: Well-balanced, good instruction following

### Mistral (Mistral AI)
- **Best for**: Code generation, technical writing, structured output
- **Size**: 7B - 123B parameters
- **Speed**: Fast to moderate
- **Strengths**: Efficient, good at following constraints

### DeepSeek Coder (DeepSeek)
- **Best for**: Code completion, debugging, code review
- **Size**: 1.3B - 33B parameters
- **Speed**: Very fast (small models)
- **Strengths**: Specialized for coding tasks, multilingual

### Qwen 2.5 (Alibaba)
- **Best for**: Multilingual tasks, math, reasoning
- **Size**: 0.5B - 72B parameters
- **Speed**: Very fast to moderate
- **Strengths**: Strong reasoning, good at structured output

## φ-Bounded Confidence

All OSS LLM adapters return responses with max φ⁻² (38.2%) confidence:

```javascript
const response = await llama.complete('Is this correct?');
console.log(response.confidence); // Max 0.382 (38.2%)
```

This is intentional — CYNIC trusts Anthropic models more (max φ⁻¹ = 61.8%), and OSS models less.

## Consensus Mode

Use multiple Ollama models for consensus voting:

```javascript
const router = getUnifiedLLMRouter();

const result = await router.call('Should we refactor this code?', {
  strategy: 'consensus',
  quorum: 0.618, // φ⁻¹ threshold
});

// Result includes votes from all available adapters
console.log(result.consensus); // true/false
console.log(result.agreement); // 0.75 (75% agreement)
```

## Performance Optimization

### 1. Model Selection by Task

```javascript
// Simple tasks: smallest/fastest model
const llama_small = createLlamaValidator({ model: 'llama3.2:1b' });

// Code tasks: DeepSeek (specialized)
const coder = createDeepSeekValidator();

// Complex reasoning: largest model you can run
const llama_large = createLlamaValidator({ model: 'llama3.2:90b' });
```

### 2. Caching

CYNIC's DataPipeline automatically caches LLM responses:

```javascript
import { getDataPipeline } from '@cynic/node/orchestration/data-pipeline';

const pipeline = getDataPipeline();

// First call: hits LLM
const result1 = await llama.complete('What is phi?');

// Second call: cache hit (if same prompt)
const result2 = await llama.complete('What is phi?'); // Instant
```

### 3. Parallel Requests

Process multiple requests in parallel:

```javascript
const prompts = [
  'Explain phi',
  'Explain golden ratio',
  'Explain fibonacci',
];

const results = await Promise.all(
  prompts.map(p => llama.complete(p))
);
```

## Troubleshooting

### Ollama Not Running

```bash
# Start Ollama service
ollama serve

# Or check if already running
ps aux | grep ollama
```

### Model Not Found

```bash
# List installed models
ollama list

# Pull missing model
ollama pull <model-name>
```

### Slow Response

```bash
# Check system resources
htop

# Consider smaller model
ollama pull llama3.2:1b  # 1B parameter version
```

### Connection Refused

```bash
# Check Ollama is listening on correct port
curl http://localhost:11434/api/tags

# If using custom endpoint, update configuration
OLLAMA_ENDPOINT=http://custom-host:11434
```

## See Also

- [Ollama Documentation](https://github.com/ollama/ollama)
- [Ollama Model Library](https://ollama.com/library)
- [CYNIC LLM Router](../../docs/architecture/llm-routing.md)
- [UnifiedLLMRouter](../node/src/orchestration/unified-llm-router.js)
