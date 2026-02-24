from typing import Protocol, AsyncIterator  
from dataclasses import dataclass  
  
@dataclass  
class LLMResponse:  
    content: str  
    model: str  
    provider: str  
    prompt_tokens: int = 0  
    completion_tokens: int = 0  
    cost_usd: float = 0.0  
    latency_ms: float = 0.0  
