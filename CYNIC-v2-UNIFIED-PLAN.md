# CYNIC v2 - Plan d'Implémentation Unifié

## 🎯 Vision
CYNIC = Agent + IDE + Orchestrateur + Modèle + OS

## ⚠️ Principe Fondamental
**RIEN N'EST Gratuit** - CYNIC connaît les coûts réels

- Claude Code: $20/mois (subscription)
- Ollama: GPU electricity + hardware amortized
- API providers: prix réel par token
- AirLLM: SSD I/O + CPU costs

---

## 📋 Todo List

### Phase 1: LLM Layer (Intelligent Switch)
- [x] 1.1 Unifier les adapters (4→1 intelligent switch) ✅ DONE
- [x] 1.2 Ajouter WebSocketClaudeAdapter ✅ DONE
- [ ] 1.3 Implémenter OllamaClaudeAdapter (Anthropic API compatible)
- [x] 1.4 Intelligent Switch avec REAL pricing ✅ DONE
- [x] 1.5 PricingOracle - coûts temps réel ✅ DONE

### Phase 2: Retrieval Layer
- [x] 2.1 Ajouter PageIndex reasoning-based retrieval ✅ DONE
- [ ] 2.2 Créer hybrid: Qdrant + PageIndex

### Phase 3: Orchestration Layer
- [ ] 3.1 Implémenter Prometheus pattern (planning)
- [ ] 3.2 Implémenter Atlas pattern (execution)
- [ ] 3.3 Intégrer avec KabbalisticRouter existant

### Phase 4: Learning Layer (Fine-tuning)
- [ ] 4.1 Créer pipeline dataset depuis learning events
- [ ] 4.2 Implémenter LoRA fine-tuning
- [ ] 4.3 Connecter adapter weights à CYNIC

### Phase 5: Infra
- [ ] 5.1 WebSocket server complet
- [ ] 5.2 CLI unifié
- [ ] 5.3 Docker optimization

---

## 🔗 Inspirations
- Vibe Companion (WebSocket protocol)
- PageIndex (reasoning-based RAG)
- oh-my-opencode (Prometheus→Atlas)
- LoRA/QLoRA (fine-tuning sans GPU)
