# CYNIC Dogs Orchestrator - System Diagram

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CYNIC Orchestrator                              │
│                        (cynic-omniscient Python)                           │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
          ┌───────────────────────────┼───────────────────────────┐
          │                           │                           │
          ▼                           ▼                           ▼
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│   PERCEPTION    │      │     ADAPTERS    │      │      JUDGE      │
│   (see all)     │      │    (LLM calls)  │      │   (36 dim)      │
├─────────────────┤      ├─────────────────┤      ├─────────────────┤
│ FilesystemWatch │      │  OllamaAdapter  │      │  JudgeEngine    │
│ NetworkMonitor  │─────▶│  AnthropicAdapt │─────▶│  set_llm_adapter│
│ ProcessMonitor  │      │  (real calls)   │      │  evaluate()     │
└─────────────────┘      └─────────────────┘      └─────────────────┘
                                      │
                                      ▼
                        ┌─────────────────────────┐
                        │    DOG COLLECTIVE      │
                        │    (11 Sefirot)        │
                        └─────────────────────────┘
                                      │
     ┌────────┬────────┬────────┬─────┴─────┬────────┬────────┬────────┐
     │        │        │        │           │        │        │        │
     ▼        ▼        ▼        ▼           ▼        ▼        ▼        ▼
┌────────┐┌──────┐┌───────┐┌────────┐ ┌────────┐┌──────┐┌───────┐┌────────┐
│ CYNIC  ││ SAGE ││ANALYST││SCHOLAR │ │ARCHIT. ││GUARD. ││ORACLE ││  ...   │
│ Keter  ││Choch.││ Binah ││  Daat  │ │ Chesed ││Gevurah││Tiferet││        │
│  meta  ││wisdom││analysis││knowledge││ architect││guardian││oracle ││        │
└────────┘└──────┘└───────┘└────────┘ └────────┘└──────┘└───────┘└────────┘
     ▲                                                                    │
     │        ┌─────────────────────────────────────────────────────┐     │
     │        │           Dog.invoke_dogs_with_synthesis()          │     │
     │        │                                                      │     │
     │        │  1. Invoke 10 Dogs (parallel)                      │     │
     │        │     - Each calls: self._call_llm(prompt)            │     │
     │        │     - Uses system_prompt for role                   │     │
     │        │     - Returns DogResult with latency                │     │
     │        │                                                      │     │
     │        │  2. CYNIC Dog synthesizes                           │     │
     │        │     - Receives all dog_results in context           │     │
     │        │     - Calls _call_llm with synthesis prompt         │     │
     │        │     - Returns final answer                          │     │
     │        └──────────────────────────────────────────────────────┘     │
     │                                                                    │
     └────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │     OUTPUT          │
                    ├─────────────────────┤
                    │ LLMResponse         │
                    │ + JudgmentResult    │
                    │ + DogResult[]       │
                    └─────────────────────┘
```

## Dog Details - What Each One Does (Real LLM Calls)

| Dog | Sefirot | System Prompt Role | LLM Call |
|-----|---------|-------------------|-----------|
| **CYNIC** | Keter | Meta-cognition, synthesis | `synthesize(all_dog_results)` |
| **Sage** | Chochmah | Wisdom, intuition | `Provide wisdom and intuitive insight on: {input}` |
| **Analyst** | Binah | Deep analysis | `Analyze and decompose this topic: {input}` |
| **Scholar** | Daat | Factual knowledge | `Research and provide factual knowledge about: {input}` |
| **Architect** | Chesed | System design | `Design a system architecture for: {input}` |
| **Guardian** | Gevurah | Security validation | `Validate and check for security concerns: {input}` |
| **Oracle** | Tiferet | Prediction | `Predict likely outcomes and forecast: {input}` |
| **Scout** | Netzach | Exploration | `Explore and discover new perspectives: {input}` |
| **Deployer** | Hod | Implementation | `Provide implementation steps and execution plan: {input}` |
| **Janitor** | Yesod | Maintenance | `Identify maintenance needs and cleanup: {input}` |
| **Cartographer** | Malkhut | Mapping | `Map out the context and provide overview: {input}` |

## Key Classes

```python
# Base Dog - each makes REAL LLM calls
class Dog(ABC):
    def set_adapter(self, adapter: LLMAdapter)  # Connect LLM
    async def process(input, context) -> DogResult  # Process with LLM
    async def _call_llm(prompt) -> str  # REAL LLM call

# Concrete Dog Example
class SageDog(Dog):
    @property
    def system_prompt(self) -> str:
        return "You are the Sage (Chochmah), embodying divine wisdom..."
    
    async def process(self, input_data, context):
        return await self._call_llm(f"Provide wisdom on: {input_data}")

# Orchestrator
class CYNIC:
    def register_adapter(self, name, adapter):
        # Auto-connects adapter to ALL dogs
        for dog in self.dogs.dogs:
            dog.set_adapter(adapter)
    
    async def invoke_dogs_with_synthesis(self, input):
        # 1. Call 10 dogs
        # 2. CYNIC synthesizes
        # 3. Returns (dog_results, synthesis)
```

## Confidence φ-Bounded

Each Dog returns confidence bounded by φ (0.618):
- Sage: 0.65 (wisdom, higher)
- Guardian: 0.63 (security)
- CYNIC: 0.618 (meta, φ-max)
- Analyst: 0.62
- Deployer: 0.61
- Janitor: 0.59
- Scholar: 0.58
- Cartographer: 0.56
- Scout: 0.57
- Oracle: 0.55 (prediction, lower)

**No mocks - Real LLM calls with latency tracking.**
