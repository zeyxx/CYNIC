"""
CYNIC LLM — Universal LLM adapter framework with provider support and benchmarking.

Abstracts away LLM provider differences (Claude, Gemini, Ollama) behind a
unified interface, enabling seamless provider swapping and multi-provider fallback.

Providers:
    claude: Anthropic Claude API
    gemini: Google Gemini API
    ollama: Local Ollama server
    openai: OpenAI API (fallback)

Components:
    providers: Provider implementations (BaseProvider subclasses)
    adapters: Protocol adapters for unified calling convention
    registry: LLMRegistry for provider lifecycle management
    benchmarking: Performance metrics and cost tracking

Typical usage:
    from cynic.llm import LLMRegistry
    registry = LLMRegistry()
    registry.register_provider('claude', api_key='...')
    response = await registry.prompt('claude', 'Tell me about consciousness')

See Also:
    cynic.cognition.cortex.handlers: Handlers that use LLM adapters
    cynic.api.handlers.intelligence: Intelligence gathering with LLMs
"""
