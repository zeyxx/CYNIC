"""
CYNIC Ollama LLM Provider — Local LLM model adapter.

Provides adapter for locally-run Ollama models, enabling offline LLM judgment
without API dependencies or costs. Perfect for development and on-premise deployments.

Supported Models:
    llama2: Meta's Llama 2 (13B/7B)
    mistral: Mistral 7B/8x7B
    neural-chat: Intel optimized model
    Custom: Any Ollama-compatible model

Features:
    Zero API costs (local execution)
    Offline operation (no internet required)
    Privacy-preserving (no external API calls)
    Customizable quantization and parameters
    Streaming response support

Typical usage:
    from cynic.llm.providers import OllamaProvider
    provider = OllamaProvider(base_url='http://localhost:11434')
    response = await provider.prompt('llama2', 'Is this proposal fair?')

See Also:
    cynic.llm.providers: Other provider implementations (Claude, Gemini)
    cynic.llm.registry: Provider registration and lifecycle
"""