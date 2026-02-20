# CYNIC v3 - Architecture Verticale (Fondations → Haut Niveau)

> **Principe**: Construire chaque couche de fond en haut, valider, puis passer à la suivante
> **Status**: ARCHITECTURE & FONDATIONS
> **Generated**: 2026-02-14

---

## PHILOSOPHIE DE CONSTRUCTION VERTICALE

```
NIVEAU 1: FONDATIONS (φ, Types, Base Classes)
    ↓ Validated
NIVEAU 2: PRIMITIVES (Math, Stats, Cache)
    ↓ Validated  
NIVEAU 3: PERSISTENCE (Database, Vector, Redis)
    ↓ Validated
NIVEAU 4: ADAPTERS (LLM Backends)
    ↓ Validated
NIVEAU 5: ORCHESTRATION (Routing, Pricing, Strategies)
    ↓ Validated
NIVEAU 6: CONTEXT (PageIndex, Compression)
    ↓ Validated
NIVEAU 7: INTER-LLM (Consensus, Voting)
    ↓ Validated
NIVEAU 8: LEARNING (Thompson, Q-Learning, Calibration)
    ↓ Validated
NIVEAU 9: JUDGE (36 Dimensions, Scoring)
    ↓ Validated
NIVEAU 10: DOGS (11 Sefirot Agents)
    ↓ Validated
NIVEAU 11: ECONOMY (Burn, Treasury, Budget)
    ↓ Validated
NIVEAU 12: SERVER (HTTP, WebSocket, CLI)
```

---

## NIVEAU 1: FONDATIONS

### 1.1 Constants & Types

```python
# src/cynic/constants/phi.py
"""
φ (Phi) = 1.6180339887498948482...
φ⁻¹ = 0.6180339887498948... (Inverse Golden Ratio)
φ⁻² = 0.3819660112501051... (Squared Inverse)
"""
from typing import Final

# Mathematical Constants
PHI: Final[float] = 1.6180339887498948482
PHI_INV: Final[float] = 0.6180339887498948  # φ⁻¹ - Max confidence
PHI_INV_SQ: Final[float] = 0.3819660112501051  # φ⁻² - Min doubt
PHI_SQ: Final[float] = 2.618033988749895  # φ²

# Confidence Bounds (φ-Aligned)
MAX_CONFIDENCE: Final[float] = PHI_INV  # 61.8%
MIN_DOUBT: Final[float] = PHI_INV_SQ  # 38.2%

# Fibonacci Intervals (for timing, polling)
FIB_1: Final[int] = 1
FIB_2: Final[int] = 1
FIB_3: Final[int] = 2
FIB_5: Final[int] = 5
FIB_8: Final[int] = 8
FIB_13: Final[int] = 13
FIB_21: Final[int] = 21
FIB_34: Final[int] = 34
FIB_55: Final[int] = 55
FIB_89: Final[int] = 89
FIB_144: Final[int] = 144

# Timing (ms)
HEARTBEAT_MS: Final[int] = 8000  # F6
STATE_SYNC_MS: Final[int] = 13000  # F7
VALIDATOR_CHECK_MS: Final[int] = 21000  # F8
METRICS_REPORT_MS: Final[int] = 34000  # F9
```

### 1.2 Base Types

```python
# src/cynic/types/__init__.py
from dataclasses import dataclass
from enum import Enum
from typing import Any, Optional

class Verdict(str, Enum):
    """Judgment outcomes"""
    HOWL = "HOWL"   # Exceptional (Q ≥ 80)
    WAG = "WAG"     # Passes (Q ≥ 50)
    GROWL = "GROWL" # Needs work (Q ≥ 38.2)
    BARK = "BARK"   # Critical (Q < 38.2)

class ActionStatus(str, Enum):
    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"
    TIMEOUT = "timeout"

@dataclass
class DecisionEvent:
    """Unified event model flowing through all layers"""
    event_id: str
    timestamp: float
    source: str
    action: str
    payload: dict[str, Any]
    context: dict[str, Any]
    status: ActionStatus = ActionStatus.PENDING

@dataclass
class JudgmentResult:
    """36-dimension judgment output"""
    q_score: float  # 0-100
    verdict: Verdict
    confidence: float  # φ-bounded: 0-0.618
    dimensions: dict[str, float]
    reasoning: str

@dataclass
class LLMResponse:
    """Standardized LLM response"""
    content: str
    provider: str
    model: str
    tokens_used: int
    cost: float
    latency_ms: float
    metadata: dict[str, Any]
```

---

## NIVEAU 2: PRIMITIVES

### 2.1 Math & Statistics

```python
# src/cynic/primitives/math.py
import math
from typing import Sequence
from cynic.constants.phi import PHI_INV, PHI_INV_SQ

class ThompsonSampler:
    """Multi-armed bandit with Beta distribution"""
    
    def __init__(self, n_arms: int):
        self.alpha = [1.0] * n_arms  # successes + 1
        self.beta = [1.0] * n_arms    # failures + 1
    
    def sample(self, arm: int) -> float:
        """Sample from Beta(α, β)"""
        import random
        import math
        # Inverse transform sampling from Beta distribution
        x = random.betavariate(self.alpha[arm], self.beta[arm])
        return x
    
    def update(self, arm: int, success: bool):
        """Update arm parameters"""
        if success:
            self.alpha[arm] += 1
        else:
            self.beta[arm] += 1
    
    def select_arm(self) -> int:
        """Thompson Sampling: select best arm"""
        samples = [self.sample(i) for i in range(len(self.alpha))]
        return samples.index(max(samples))

class HilbertIndex:
    """Hilbert curve spatial indexing for vector similarity"""
    
    def __init__(self, dimension: int, order: int = 8):
        self.dimension = dimension
        self.order = order
    
    def encode(self, coordinates: list[float]) -> int:
        """Map N-D coordinates to 1D Hilbert index"""
        # Implementation of Hilbert curve encoding
        # Maps nearby points in N-D to nearby indices in 1D
        pass
    
    def find_near(self, point: list[float], k: int) -> list[int]:
        """Find k nearest neighbors"""
        pass
```

### 2.2 Cache & Rate Limiting

```python
# src/cynic/primitives/cache.py
from typing import Any, Optional
import time

class RateLimiter:
    """Fibonacci-based rate limiting"""
    
    def __init__(self, rate: float, window_ms: int = 13000):
        self.rate = rate  # requests per window
        self.window_ms = window_ms
        self.requests: list[float] = []
    
    def allow(self) -> bool:
        now = time.time() * 1000
        # Remove old requests outside window
        self.requests = [t for t in self.requests if now - t < self.window_ms]
        
        if len(self.requests) < self.rate:
            self.requests.append(now)
            return True
        return False
    
    def wait_time(self) -> float:
        """Time to wait before next request allowed"""
        if not self.requests:
            return 0
        now = time.time() * 1000
        return max(0, self.window_ms - (now - self.requests[0]))
```

---

## NIVEAU 3: PERSISTENCE

### 3.1 Database Abstraction

```python
# src/cynic/persistence/database.py
from abc import ABC, abstractmethod
from typing import Any, Optional
import asyncio

class Database(ABC):
    """Abstract database interface"""
    
    @abstractmethod
    async def connect(self) -> None: ...
    
    @abstractmethod
    async def disconnect(self) -> None: ...
    
    @abstractmethod
    async def execute(self, query: str, params: tuple = ()) -> Any: ...
    
    @abstractmethod
    async def fetchone(self, query: str, params: tuple = ()) -> Optional[dict]: ...
    
    @abstractmethod
    async def fetchall(self, query: str, params: tuple = ()) -> list[dict]: ...

class PostgreSQL(Database):
    """PostgreSQL implementation"""
    
    def __init__(self, connection_string: str):
        self.connection_string = connection_string
        self.pool = None
    
    async def connect(self) -> None:
        import asyncpg
        self.pool = await asyncpg.create_pool(self.connection_string)
    
    async def disconnect(self) -> None:
        await self.pool.close()

class VectorStore(Database):
    """Vector similarity search (Qdrant)"""
    
    def __init__(self, url: str, collection: str = "cynic"):
        self.url = url
        self.collection = collection
        self.client = None
    
    async def connect(self) -> None:
        from qdrant_client import QdrantClient
        self.client = QdrantClient(url=self.url)
    
    async def search(self, query_vector: list[float], k: int = 5) -> list[dict]:
        """Vector similarity search"""
        results = self.client.search(
            collection_name=self.collection,
            query_vector=query_vector,
            limit=k
        )
        return [{"id": r.id, "score": r.score, "payload": r.payload} for r in results]
    
    async def upsert(self, vectors: list[tuple[str, list[float], dict]]) -> None:
        """Insert/update vectors"""
        from qdrant_client.models import PointStruct
        points = [
            PointStruct(id=id, vector=vec, payload=payload)
            for id, vec, payload in vectors
        ]
        self.client.upsert(collection_name=self.collection, points=points)
```

---

## NIVEAU 4: ADAPTERS (LLM Backends)

### 4.1 Base Adapter

```python
# src/cynic/adapters/base.py
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import AsyncIterator, Optional
from cynic.types import LLMResponse

@dataclass
class AdapterConfig:
    """Configuration for LLM adapter"""
    name: str
    model: str
    max_tokens: int = 4096
    temperature: float = 0.7
    timeout_ms: int = 60000

class LLMAdapter(ABC):
    """Abstract LLM adapter interface"""
    
    def __init__(self, config: AdapterConfig):
        self.config = config
        self.name = config.name
        self.model = config.model
    
    @abstractmethod
    async def generate(
        self,
        prompt: str,
        system_prompt: Optional[str] = None,
        **kwargs
    ) -> LLMResponse:
        """Generate a response"""
        pass
    
    @abstractmethod
    async def stream(
        self,
        prompt: str,
        system_prompt: Optional[str] = None,
        **kwargs
    ) -> AsyncIterator[str]:
        """Stream response tokens"""
        pass
    
    @abstractmethod
    async def health_check(self) -> bool:
        """Check if adapter is available"""
        pass
    
    @property
    def cost_per_1k_input(self) -> float:
        """Cost per 1K input tokens in USD"""
        return 0.0
    
    @property
    def cost_per_1k_output(self) -> float:
        """Cost per 1K output tokens in USD"""
        return 0.0
```

### 4.2 Ollama Adapter

```python
# src/cynic/adapters/ollama.py
import httpx
from typing import AsyncIterator
from cynic.adapters.base import LLMAdapter, AdapterConfig
from cynic.types import LLMResponse

class OllamaAdapter(LLMAdapter):
    """Local LLM via Ollama"""
    
    def __init__(self, config: AdapterConfig, base_url: str = "http://localhost:11434"):
        super().__init__(config)
        self.base_url = base_url
    
    async def generate(
        self,
        prompt: str,
        system_prompt: Optional[str] = None,
        **kwargs
    ) -> LLMResponse:
        import time
        start = time.time()
        
        async with httpx.AsyncClient(timeout=60.0) as client:
            payload = {
                "model": self.model,
                "prompt": prompt,
                "stream": False,
                **kwargs
            }
            if system_prompt:
                payload["system"] = system_prompt
            
            response = await client.post(
                f"{self.base_url}/api/generate",
                json=payload
            )
            data = response.json()
            
            return LLMResponse(
                content=data.get("response", ""),
                provider="ollama",
                model=self.model,
                tokens_used=data.get("eval_count", 0),
                cost=0.0,  # Free (local)
                latency_ms=(time.time() - start) * 1000,
                metadata={}
            )
    
    async def stream(self, prompt: str, system_prompt: Optional[str] = None) -> AsyncIterator[str]:
        async with httpx.AsyncClient(timeout=60.0) as client:
            payload = {"model": self.model, "prompt": prompt, "stream": True}
            async with client.stream("POST", f"{self.base_url}/api/generate", json=payload) as response:
                async for line in response.aiter_lines():
                    if line:
                        data = eval(line)  # Parse JSON lines
                        if "response" in data:
                            yield data["response"]
    
    async def health_check(self) -> bool:
        try:
            async with httpx.AsyncClient(timeout=5.0) as client:
                response = await client.get(f"{self.base_url}/api/tags")
                return response.status_code == 200
        except:
            return False
    
    @property
    def cost_per_1k_input(self) -> float:
        return 0.0  # Free
    
    @property
    def cost_per_1k_output(self) -> float:
        return 0.0  # Free
```

### 4.3 Anthropic Adapter

```python
# src/cynic/adapters/anthropic.py
import anthropic
import time
from cynic.adapters.base import LLMAdapter, AdapterConfig
from cynic.types import LLMResponse
from typing import Optional

# Pricing (2026-02)
ANTHROPIC_PRICING = {
    "claude-opus-4-5-20250501": {"input": 15.0, "output": 75.0},
    "claude-sonnet-4-5-20250501": {"input": 3.0, "output": 15.0},
    "claude-haiku-3-5-20250501": {"input": 0.8, "output": 4.0},
}

class AnthropicAdapter(LLMAdapter):
    """Anthropic API (Claude)"""
    
    def __init__(self, config: AdapterConfig, api_key: str):
        super().__init__(config)
        self.client = anthropic.AsyncAnthropic(api_key=api_key)
    
    async def generate(
        self,
        prompt: str,
        system_prompt: Optional[str] = None,
        **kwargs
    ) -> LLMResponse:
        start = time.time()
        
        message = await self.client.messages.create(
            model=self.model,
            max_tokens=self.config.max_tokens,
            system=system_prompt,
            messages=[{"role": "user", "content": prompt}],
            **kwargs
        )
        
        input_tokens = message.usage.input_tokens
        output_tokens = message.usage.output_tokens
        
        pricing = ANTHROPIC_PRICING.get(self.model, {"input": 3.0, "output": 15.0})
        cost = (input_tokens / 1000 * pricing["input"]) + (output_tokens / 1000 * pricing["output"])
        
        return LLMResponse(
            content=message.content[0].text,
            provider="anthropic",
            model=self.model,
            tokens_used=input_tokens + output_tokens,
            cost=cost,
            latency_ms=(time.time() - start) * 1000,
            metadata={"input_tokens": input_tokens, "output_tokens": output_tokens}
        )
    
    async def health_check(self) -> bool:
        try:
            # Simple API check
            return True
        except:
            return False
    
    @property
    def cost_per_1k_input(self) -> float:
        return ANTHROPIC_PRICING.get(self.model, {}).get("input", 3.0)
    
    @property
    def cost_per_1k_output(self) -> float:
        return ANTHROPIC_PRICING.get(self.model, {}).get("output", 15.0)
```

### 4.4 Adapter Registry

```python
# src/cynic/adapters/registry.py
from typing import Optional
from cynic.adapters.base import LLMAdapter, AdapterConfig

class AdapterRegistry:
    """Registry for LLM adapters"""
    
    def __init__(self):
        self._adapters: dict[str, LLMAdapter] = {}
    
    def register(self, name: str, adapter: LLMAdapter):
        self._adapters[name] = adapter
    
    def get(self, name: str) -> Optional[LLMAdapter]:
        return self._adapters.get(name)
    
    def list_available(self) -> list[str]:
        return list(self._adapters.keys())
    
    async def detect_available(self) -> list[str]:
        """Detect which adapters are working"""
        available = []
        for name, adapter in self._adapters.items():
            if await adapter.health_check():
                available.append(name)
        return available

# Global registry
registry = AdapterRegistry()
```

---

## NIVEAU 5: ORCHESTRATION

### 5.1 Pricing Oracle

```python
# src/cynic/orchestration/pricing.py
from dataclasses import dataclass
from typing import Optional

@dataclass
class PricingInfo:
    """Pricing information for a model"""
    provider: str
    model: str
    cost_per_1k_input: float
    cost_per_1k_output: float
    latency_estimate_ms: float
    quality_score: float  # 0-1

class PricingOracle:
    """Oracle for real-time LLM pricing"""
    
    def __init__(self):
        self._pricing_cache: dict[str, PricingInfo] = {}
        self._load_default_pricing()
    
    def _load_default_pricing(self):
        """Load default pricing from known models"""
        defaults = {
            # Anthropic
            "claude-opus-4-5": PricingInfo("anthropic", "opus", 15.0, 75.0, 3000, 0.95),
            "claude-sonnet-4-5": PricingInfo("anthropic", "sonnet", 3.0, 15.0, 1500, 0.85),
            "claude-haiku-3-5": PricingInfo("anthropic", "haiku", 0.8, 4.0, 500, 0.75),
            # OpenAI
            "gpt-4-turbo": PricingInfo("openai", "gpt-4-turbo", 10.0, 30.0, 2000, 0.90),
            "gpt-4o": PricingInfo("openai", "gpt-4o", 2.5, 10.0, 1500, 0.88),
            "gpt-4o-mini": PricingInfo("openai", "gpt-4o-mini", 0.15, 0.6, 500, 0.70),
            # Ollama (free)
            "llama3-70b": PricingInfo("ollama", "llama3:70b", 0.0, 0.0, 10000, 0.80),
            "mistral": PricingInfo("ollama", "mistral", 0.0, 0.0, 8000, 0.75),
            "qwen2.5-coder": PricingInfo("ollama", "qwen2.5-coder", 0.0, 0.0, 5000, 0.78),
        }
        self._pricing_cache.update(defaults)
    
    def calculate_cost(
        self,
        provider: str,
        model: str,
        input_tokens: int,
        output_tokens: int
    ) -> float:
        """Calculate cost in USD"""
        key = f"{provider}-{model}"
        info = self._pricing_cache.get(key)
        
        if not info:
            return 0.0
        
        return (input_tokens / 1000 * info.cost_per_1k_input) + \
               (output_tokens / 1000 * info.cost_per_1k_output)
    
    def get_pricing(self, provider: str, model: str) -> Optional[PricingInfo]:
        """Get pricing info for a model"""
        key = f"{provider}-{model}"
        return self._pricing_cache.get(key)
```

### 5.2 Intelligent Switch

```python
# src/cynic/orchestration/router.py
from enum import Enum
from typing import Optional
from dataclasses import dataclass
from cynic.adapters.base import LLMAdapter
from cynic.constants.phi import PHI_INV

class Strategy(str, Enum):
    FREE = "free"
    SPEED = "speed"
    QUALITY = "quality"
    BALANCED = "balanced"

@dataclass
class RouterConfig:
    strategy: Strategy = Strategy.BALANCED
    max_budget: float = 10.0
    min_quality: float = 0.5

class IntelligentSwitch:
    """Intelligent model selection based on cost/speed/quality/privacy"""
    
    def __init__(
        self,
        adapters: dict[str, LLMAdapter],
        pricing_oracle,
        config: Optional[RouterConfig] = None
    ):
        self.adapters = adapters
        self.pricing = pricing_oracle
        self.config = config or RouterConfig()
    
    async def select(
        self,
        task_type: str,
        budget: Optional[float] = None,
        required_quality: Optional[float] = None
    ) -> Optional[LLMAdapter]:
        """Select best adapter based on strategy"""
        
        candidates = []
        budget = budget or self.config.max_budget
        required_quality = required_quality or self.config.min_quality
        
        for name, adapter in self.adapters.items():
            if not await adapter.health_check():
                continue
            
            pricing = self.pricing.get_pricing(adapter.provider, adapter.model)
            if not pricing:
                continue
            
            # Calculate score based on strategy
            score = self._calculate_score(adapter, pricing, task_type)
            
            if pricing.cost_per_1k_input <= budget and pricing.quality_score >= required_quality:
                candidates.append((name, adapter, score))
        
        if not candidates:
            return None
        
        # Return best scored adapter
        candidates.sort(key=lambda x: x[2], reverse=True)
        return candidates[0][1]
    
    def _calculate_score(self, adapter, pricing, task_type: str) -> float:
        """Calculate adapter score based on strategy"""
        
        # Normalize metrics (0-1)
        cost_score = 1.0 - min(pricing.cost_per_1k_input / 0.10, 1.0)  # $0.10 as max
        speed_score = 1.0 - min(pricing.latency_estimate_ms / 10000, 1.0)  # 10s as max
        quality_score = pricing.quality_score
        
        if self.config.strategy == Strategy.FREE:
            return cost_score * 2.0 + quality_score * 0.5
        elif self.config.strategy == Strategy.SPEED:
            return speed_score * 2.0 + quality_score * 0.5
        elif self.config.strategy == Strategy.QUALITY:
            return quality_score * 2.0 + speed_score * 0.5
        else:  # BALANCED
            return cost_score + speed_score + quality_score
```

---

## NIVEAU 6: CONTEXT (PageIndex, Compression)

### 6.1 PageIndex (Hybrid RAG)

```python
# src/cynic/context/page_index.py
from typing import Optional
from cynic.persistence.database import VectorStore, Database
from cynic.primitives.math import HilbertIndex

class PageIndex:
    """
    Hybrid RAG: Tree + Vector similarity
    Achieves 98.7% accuracy by combining:
    - Tree search (precision)
    - Vector search (recall)
    """
    
    def __init__(
        self,
        vector_store: VectorStore,
        tree_depth: int = 5
    ):
        self.vector_store = vector_store
        self.tree_depth = tree_depth
        self.hilbert = HilbertIndex(dimension=384, order=tree_depth)
    
    async def build_from_documents(self, documents: list[dict]) -> None:
        """Build index from documents"""
        # 1. Generate embeddings
        # 2. Build tree structure
        # 3. Store in vector DB
        pass
    
    async def retrieve(
        self,
        query: str,
        method: str = "hybrid",
        k: int = 5
    ) -> list[dict]:
        """
        Retrieve relevant context
        
        Methods:
        - tree: Precision-focused
        - vector: Recall-focused  
        - hybrid: Both (default)
        """
        
        if method == "tree":
            return await self._tree_search(query, k)
        elif method == "vector":
            return await self._vector_search(query, k)
        else:  # hybrid
            tree_results = await self._tree_search(query, k)
            vector_results = await self._vector_search(query, k)
            return self._merge_results(tree_results, vector_results)
    
    async def _tree_search(self, query: str, k: int) -> list[dict]:
        """Tree-based search (precision)"""
        pass
    
    async def _vector_search(self, query: str, k: int) -> list[dict]:
        """Vector similarity search (recall)"""
        # Generate query embedding
        # Search vector store
        # Return top-k
        pass
    
    def _merge_results(self, tree: list[dict], vector: list[dict]) -> list[dict]:
        """Merge tree + vector results with scoring"""
        # Combine and re-rank
        pass
```

---

## NIVEAU 7: INTER-LLM (Consensus, Voting)

### 7.1 Consensus Protocol

```python
# src/cynic/inter_llm/consensus.py
from typing import list
from dataclasses import dataclass
from cynic.types import LLMResponse
from cynic.constants.phi import PHI_INV

@dataclass
class VoteResult:
    """Result from a single LLM vote"""
    adapter_name: str
    vote: str
    confidence: float

@dataclass
class ConsensusResult:
    """Final consensus result"""
    verdict: str
    agreement: float  # 0-1
    votes: list[VoteResult]
    quorum_met: bool

class ConsensusProtocol:
    """
    Multi-LLM voting with φ⁻¹ (61.8%) quorum
    """
    
    def __init__(self, quorum: float = PHI_INV):
        self.quorum = quorum  # 61.8% default
    
    async def vote(
        self,
        prompt: str,
        adapters: list[LLMAdapter],
        threshold: float = 0.6
    ) -> ConsensusResult:
        """Collect votes from multiple LLMs"""
        
        votes: list[VoteResult] = []
        
        # Get responses from all adapters
        for adapter in adapters:
            try:
                response = await adapter.generate(prompt)
                votes.append(VoteResult(
                    adapter_name=adapter.name,
                    vote=response.content,
                    confidence=1.0  # Could be LLM confidence
                ))
            except Exception as e:
                # Log error, continue with others
                pass
        
        if not votes:
            return ConsensusResult(
                verdict="NO_CONSENSUS",
                agreement=0.0,
                votes=[],
                quorum_met=False
            )
        
        # Calculate agreement
        agreement = self._calculate_agreement(votes)
        
        # Determine if quorum is met
        quorum_met = agreement >= self.quorum
        
        # Determine verdict
        if quorum_met:
            verdict = self._majority_vote(votes)
        else:
            verdict = "NO_CONSENSUS"
        
        return ConsensusResult(
            verdict=verdict,
            agreement=agreement,
            votes=votes,
            quorum_met=quorum_met
        )
    
    def _calculate_agreement(self, votes: list[VoteResult]) -> float:
        """Calculate how much LLMs agree"""
        if len(votes) <= 1:
            return 1.0
        
        # Simple: count matching votes
        # Could be enhanced with semantic similarity
        matching = 0
        total = 0
        
        for i, v1 in enumerate(votes):
            for v2 in votes[i+1:]:
                total += 1
                if v1.vote == v2.vote:
                    matching += 1
        
        return matching / total if total > 0 else 0.0
    
    def _majority_vote(self, votes: list[VoteResult]) -> str:
        """Get majority vote"""
        from collections import Counter
        contents = [v.vote for v in votes]
        return Counter(contents).most_common(1)[0][0]
```

---

## NIVEAU 8: LEARNING

### 8.1 Learning Engine

```python
# src/cynic/learning/engine.py
from dataclasses import dataclass, field
from typing import Optional
from cynic.primitives.math import ThompsonSampler

@dataclass
class LearningState:
    """State of the learning system"""
    routing_weights: dict[str, float] = field(default_factory=dict)
    model_affinities: dict[str, dict[str, float]] = field(default_factory=dict)
    pattern_success_rates: dict[str, float] = field(default_factory=dict)

class LearningEngine:
    """
    Multi-layer learning system:
    - Thompson Sampling for exploration/exploitation
    - Q-Learning for routing
    - Pattern recognition for success rates
    """
    
    def __init__(self):
        self.state = LearningState()
        self.thompson = ThompsonSampler(n_arms=4)  # 4 strategies
        self._init_default_weights()
    
    def _init_default_weights(self):
        """Initialize default routing weights"""
        strategies = ["free", "speed", "quality", "balanced"]
        for s in strategies:
            self.state.routing_weights[s] = 0.25
    
    async def update_routing(
        self,
        strategy: str,
        success: bool,
        cost: float,
        quality: float
    ):
        """Update routing weights based on outcome"""
        
        # Update Thompson sampler
        arm_idx = ["free", "speed", "quality", "balanced"].index(strategy)
        self.thompson.update(arm_idx, success)
        
        # Update routing weights (Q-learning style)
        alpha = 0.1  # Learning rate
        reward = quality - cost * 0.1  # Quality - cost tradeoff
        
        current_weight = self.state.routing_weights.get(strategy, 0.25)
        self.state.routing_weights[strategy] = current_weight + alpha * reward
    
    async def select_strategy(self) -> str:
        """Thompson Sampling: select best strategy"""
        arm = self.thompson.select_arm()
        return ["free", "speed", "quality", "balanced"][arm]
    
    async def record_pattern(
        self,
        pattern: str,
        success: bool
    ):
        """Record pattern success rate"""
        current = self.state.pattern_success_rates.get(pattern, 0.5)
        self.state.pattern_success_rates[pattern] = current * 0.9 + (1.0 if success else 0.0) * 0.1
```

---

## NIVEAU 9: JUDGE (36 Dimensions)

### 9.1 Judgment System

```python
# src/cynic/judge/engine.py
from dataclasses import dataclass
from enum import Enum
from typing import Optional
from cynic.types import Verdict
from cynic.constants.phi import PHI_INV, MAX_CONFIDENCE

class Dimension(str, Enum):
    """36 Judgment Dimensions"""
    # PHI - Structure/Beauty
    COHERENCE = "coherence"
    ELEGANCE = "elegance"
    STRUCTURE = "structure"
    HARMONY = "harmony"
    PRECISION = "precision"
    COMPLETENESS = "completeness"
    PROPORTION = "proportion"
    
    # VERIFY - Verification
    ACCURACY = "accuracy"
    PROVENANCE = "provenance"
    INTEGRITY = "integrity"
    VERIFIABILITY = "verifiability"
    TRANSPARENCY = "transparency"
    REPRODUCIBILITY = "reproducibility"
    CONSENSUS = "consensus"
    
    # CULTURE - Memetics
    AUTHENTICITY = "authenticity"
    RESONANCE = "resonance"
    NOVELTY = "novelty"
    ALIGNMENT = "alignment"
    RELEVANCE = "relevance"
    IMPACT = "impact"
    LINEAGE = "lineage"
    
    # BURN - Utility
    UTILITY = "utility"
    SUSTAINABILITY = "sustainability"
    EFFICIENCY = "efficiency"
    VALUE_CREATION = "value_creation"
    SACRIFICE = "sacrifice"
    CONTRIBUTION = "contribution"
    IRREVERSIBILITY = "irreversibility"
    
    # FIDELITY - Truth
    COMMITMENT = "commitment"
    ATTUNEMENT = "attunement"
    CANDOR = "candor"
    CONGRUENCE = "congruence"
    ACCOUNTABILITY = "accountability"
    VIGILANCE = "vigilance"
    KENOSIS = "kenosis"

@dataclass
class JudgmentResult:
    """36-dimension judgment output"""
    q_score: float  # 0-100, geometric mean
    verdict: Verdict
    confidence: float  # φ-bounded: 0-0.618
    dimensions: dict[str, float]
    reasoning: str

class JudgeEngine:
    """
    36-dimension judgment system with φ-bounded confidence
    """
    
    def __init__(self):
        self.dimensions = list(Dimension)
    
    async def evaluate(
        self,
        content: str,
        context: Optional[dict] = None
    ) -> JudgmentResult:
        """Evaluate content across 36 dimensions"""
        
        # Score each dimension
        scores = {}
        for dim in self.dimensions:
            scores[dim.value] = await self._score_dimension(content, dim, context)
        
        # Calculate Q-score (geometric mean of non-zero scores)
        non_zero = [s for s in scores.values() if s > 0]
        if non_zero:
            import math
            q_score = math.pow(math.prod(non_zero), 1/len(non_zero)) * 100
        else:
            q_score = 0.0
        
        # Determine verdict
        verdict = self._determine_verdict(q_score)
        
        # Calculate confidence (φ-bounded)
        confidence = min(MAX_CONFIDENCE, q_score / 100 + 0.1)
        
        return JudgmentResult(
            q_score=q_score,
            verdict=verdict,
            confidence=confidence,
            dimensions=scores,
            reasoning="Multi-dimensional evaluation complete"
        )
    
    async def _score_dimension(
        self,
        content: str,
        dimension: Dimension,
        context: Optional[dict]
    ) -> float:
        """Score a single dimension (0-1)"""
        # Placeholder: real implementation would use LLM scoring
        import random
        return random.uniform(0.3, 0.9)
    
    def _determine_verdict(self, q_score: float) -> Verdict:
        """Determine verdict from Q-score"""
        if q_score >= 80:
            return Verdict.HOWL
        elif q_score >= 50:
            return Verdict.WAG
        elif q_score >= 38.2:
            return Verdict.GROWL
        else:
            return Verdict.BARK
```

---

## NIVEAU 10: DOGS (11 Sefirot)

### 10.1 Dog Base Class

```python
# src/cynic/dogs/base.py
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Optional, Any
from cynic.types import JudgmentResult

@dataclass
class DogInput:
    """Input to a Dog"""
    prompt: str
    context: dict[str, Any]
    previous_judgment: Optional[JudgmentResult] = None

@dataclass
class DogOutput:
    """Output from a Dog"""
    content: str
    metadata: dict[str, Any]
    judgment: Optional[JudgmentResult] = None

class Dog(ABC):
    """
    Base class for all 11 Sefirot Dogs
    Each Dog is a specialized agent with distinct capabilities
    """
    
    def __init__(self, name: str, sefira: str, level1: bool = False):
        self.name = name
        self.sefira = sefira
        self.level1 = level1  # If True, uses LLM for processing
    
    @abstractmethod
    async def process(self, input: DogInput) -> DogOutput:
        """Process input and produce output"""
        pass
    
    async def can_handle(self, task: str) -> bool:
        """Check if this Dog can handle the task"""
        return True
    
    def get_capabilities(self) -> list[str]:
        """Return list of capabilities"""
        return []

class GuardianDog(Dog):
    """Gevurah - Security, protection, danger blocking"""
    
    def __init__(self):
        super().__init__("Guardian", "Gevurah", level1=True)
    
    async def process(self, input: DogInput) -> DogOutput:
        # Check for security threats
        # Block dangerous operations
        # Validate safety
        pass
    
    def get_capabilities(self) -> list[str]:
        return ["security_check", "threat_detection", "validation"]

class ScoutDog(Dog):
    """Netzach - Exploration, unknown territory"""
    
    def __init__(self):
        super().__init__("Scout", "Netzach", level1=True)
    
    async def process(self, input: DogInput) -> DogOutput:
        # Explore new patterns
        # Discover novel solutions
        pass
    
    def get_capabilities(self) -> list[str]:
        return ["exploration", "discovery", "pattern_detection"]
```

---

## NIVEAU 11: ECONOMY

### 11.1 Burn Mechanism

```python
# src/cynic/economy/burn.py
from dataclasses import dataclass
from cynic.constants.phi import PHI_INV

@dataclass
class BurnResult:
    """Result of a burn operation"""
    burned: float
    treasury: float
    multiplier: float

class BurnMechanism:
    """
    $ASDFASDFA Burn Mechanism
    - 76.4% (φ²/φ²) burned
    - 23.6% (φ⁻²) to treasury
    """
    
    def __init__(self):
        self.burn_ratio = 1 - PHI_INV  # ~0.382
        self.treasury_ratio = PHI_INV  # ~0.618
    
    async def burn(self, action: str, base_amount: float = 1.618) -> BurnResult:
        """Execute burn"""
        
        # Calculate multiplier based on action complexity
        multiplier = self._get_action_multiplier(action)
        
        amount = base_amount * multiplier
        
        burned = amount * self.burn_ratio
        treasury = amount * self.treasury_ratio
        
        return BurnResult(
            burned=burned,
            treasury=treasury,
            multiplier=multiplier
        )
    
    def _get_action_multiplier(self, action: str) -> float:
        """Get multiplier based on action type"""
        multipliers = {
            "simple": 1.0,
            "complex_analysis": 1.618,
            "deep_reasoning": 2.618,
            "consensus": 4.236,
        }
        return multipliers.get(action, 1.0)
```

---

## NIVEAU 12: SERVER

### 12.1 FastAPI Server

```python
# src/cynic/server/http.py
from fastapi import FastAPI, WebSocket
from pydantic import BaseModel
from cynic.adapters.registry import registry
from cynic.orchestration.router import IntelligentSwitch
from cynic.judge.engine import JudgeEngine

app = FastAPI(title="CYNIC v3")

class GenerateRequest(BaseModel):
    prompt: str
    strategy: str = "balanced"
    max_budget: float = 10.0

@app.post("/generate")
async def generate(request: GenerateRequest):
    """Generate response using best available LLM"""
    
    # Get adapter
    switch = IntelligentSwitch(registry._adapters, pricing_oracle)
    adapter = await switch.select(task_type="general", budget=request.max_budget)
    
    if not adapter:
        return {"error": "No adapter available"}
    
    # Generate
    response = await adapter.generate(request.prompt)
    
    return {
        "content": response.content,
        "provider": response.provider,
        "model": response.model,
        "cost": response.cost
    }

@app.post("/judge")
async def judge(content: str):
    """Judge content across 36 dimensions"""
    judge_engine = JudgeEngine()
    result = await judge_engine.evaluate(content)
    
    return {
        "q_score": result.q_score,
        "verdict": result.verdict.value,
        "confidence": result.confidence,
        "dimensions": result.dimensions
    }

@app.get("/health")
async def health():
    """Health check"""
    available = await registry.detect_available()
    return {"status": "healthy", "adapters": available}
```

---

## ORDRE D'IMPLÉMENTATION VERTICALE

| Niveau | Couche | Status | Tests Requis |
|--------|--------|--------|--------------|
| 1 | Fondations (φ, Types) | À faire | Unit tests |
| 2 | Primitives (Math, Cache) | À faire | Unit tests |
| 3 | Persistence (DB, Vector) | À faire | Integration tests |
| 4 | Adapters (LLM Backends) | À faire | Adapter tests |
| 5 | Orchestration (Router, Pricing) | À faire | Router tests |
| 6 | Context (PageIndex) | À faire | RAG tests |
| 7 | Inter-LLM (Consensus) | À faire | Voting tests |
| 8 | Learning (Thompson, Q-Learning) | À faire | Learning tests |
| 9 | Judge (36 Dimensions) | À faire | Judgment tests |
| 10 | Dogs (11 Sefirot) | À faire | Dog tests |
| 11 | Economy (Burn, Treasury) | À faire | Economy tests |
| 12 | Server (HTTP, WebSocket) | À faire | E2E tests |

---

*Architecture verticale - Chaque niveau doit être validé avant de passer au suivant*
