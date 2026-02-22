# CYNIC Chat/Code MVP — Phase 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Build a working Chat/Code unified interface where users talk to CYNIC and see generated code (MVP).

**Architecture:** Port existing CLI chat to web UI + wire to FastAPI endpoints + add learning event queuing. Chat logic already exists; we're integrating it into the webapp.

**Tech Stack:** TypeScript (vanilla), FastAPI (existing), PostgreSQL (events storage), WebSocket (real-time)

**Phase 1 Success Criteria:**
- ✅ User can type prompt in chat panel
- ✅ CYNIC responds with code
- ✅ Code displays in code panel
- ✅ User can give feedback (good/needs fix/reject)
- ✅ Events queued to database
- ✅ All tests passing (15+ tests)
- ✅ Works on fresh Docker container

---

## Task Breakdown (Parallel Execution Recommended)

**Group A: Frontend Structure (Tasks 1.1-1.4) — Can run in parallel**
**Group B: Backend API (Tasks 2.1-2.3) — Dependent on existing endpoints**
**Group C: Integration (Tasks 3.1-3.2) — Depends on A + B**
**Group D: Testing (Tasks 4.1-4.2) — Depends on all above**

---

## GROUP A: Frontend Structure

### Task 1.1: Create Chat Type Definitions

**Files:**
- Create: `webapp/src/types/chat.ts`
- Create: `webapp/src/types/learning.ts`
- Modify: `webapp/src/main.ts` (import types)

**Step 1: Write type definitions**

```typescript
// webapp/src/types/chat.ts
export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  code?: string;
  code_language?: string;
  judgment_id: string;
  timestamp: number;
  execution_status?: 'success' | 'error' | 'pending';
}

export interface ChatSession {
  id: string;
  created_at: number;
  messages: ChatMessage[];
}

export interface CodeOutput {
  code: string;
  language: string;
  status: 'generated' | 'error' | 'executing';
  output?: string;
  error?: string;
}

// webapp/src/types/learning.ts
export interface LearningEvent {
  id: string;
  session_id: string;
  prompt: string;
  code_generated: string;
  user_feedback: 'good' | 'needs_fix' | 'rejected' | null;
  execution_status: 'success' | 'error' | 'pending';
  timestamp: number;
  q_score?: number;
  senior_dev_status?: 'pending' | 'approved' | 'rejected';
}
```

**Step 2: Run import check**

```bash
cd webapp && npx tsc --noEmit webapp/src/types/chat.ts
# Expected: No errors
```

**Step 3: Commit**

```bash
git add webapp/src/types/chat.ts webapp/src/types/learning.ts
git commit -m "feat(types): Add ChatMessage and LearningEvent interfaces"
```

---

### Task 1.2: Create ChatPanel Component (with XSS protection)

**Files:**
- Create: `webapp/src/ui/chat-panel.ts`
- Create: `webapp/src/ui/styles/chat-panel.css`
- Create: `webapp/tests/chat-panel.test.ts`

**Key security:** Use textContent for user input, createTextNode for safety.

**Implementation:**

```typescript
// webapp/src/ui/chat-panel.ts
import { ChatMessage } from '../types/chat';

export class ChatPanel {
  private container: HTMLElement;
  private messages: ChatMessage[] = [];
  private sendCallback?: (text: string) => void;

  constructor(containerId: string = 'chat-panel') {
    this.container = document.getElementById(containerId) || document.createElement('div');
  }

  render(): HTMLElement {
    const div = document.createElement('div');
    div.className = 'chat-panel';

    // Messages container
    const msgDiv = document.createElement('div');
    msgDiv.className = 'messages';

    this.messages.forEach(msg => {
      const msgEl = document.createElement('div');
      msgEl.className = `message ${msg.role}`;

      const contentEl = document.createElement('div');
      contentEl.className = 'message-content';
      contentEl.textContent = msg.content; // SAFE: textContent

      const timeEl = document.createElement('div');
      timeEl.className = 'timestamp';
      timeEl.textContent = new Date(msg.timestamp).toLocaleTimeString();

      msgEl.appendChild(contentEl);
      msgEl.appendChild(timeEl);
      msgDiv.appendChild(msgEl);
    });

    div.appendChild(msgDiv);

    // Input area
    const inputArea = document.createElement('div');
    inputArea.className = 'input-area';

    const input = document.createElement('input');
    input.type = 'text';
    input.id = 'chat-input';
    input.placeholder = 'Type your intent...';

    const sendBtn = document.createElement('button');
    sendBtn.id = 'send-btn';
    sendBtn.textContent = 'Send';
    sendBtn.addEventListener('click', () => {
      const text = input.value.trim();
      if (text && this.sendCallback) {
        this.sendCallback(text);
        input.value = '';
      }
    });

    inputArea.appendChild(input);
    inputArea.appendChild(sendBtn);
    div.appendChild(inputArea);

    return div;
  }

  addMessage(msg: ChatMessage): void {
    this.messages.push(msg);
    this.refresh();
  }

  onSend(callback: (text: string) => void): void {
    this.sendCallback = callback;
  }

  private refresh(): void {
    this.container.innerHTML = '';
    this.container.appendChild(this.render());
  }
}
```

**Tests:**

```typescript
// webapp/tests/chat-panel.test.ts
import { ChatPanel } from '../src/ui/chat-panel';
import { ChatMessage } from '../src/types/chat';

describe('ChatPanel', () => {
  it('should render chat panel', () => {
    const panel = new ChatPanel();
    const html = panel.render();
    expect(html.querySelector('#chat-input')).toBeTruthy();
    expect(html.querySelector('#send-btn')).toBeTruthy();
  });

  it('should add message safely', () => {
    const panel = new ChatPanel();
    const msg: ChatMessage = {
      id: 'm1',
      role: 'user',
      content: '<script>alert("xss")</script>',
      judgment_id: 'j1',
      timestamp: Date.now(),
    };
    panel.addMessage(msg);
    const html = panel.render();
    expect(html.textContent).toContain('<script>');
    expect(html.querySelector('script')).toBeFalsy(); // NO script execution
  });
});
```

**Run tests:**

```bash
cd webapp && npm test -- chat-panel.test.ts
```

**Commit:**

```bash
git add webapp/src/ui/chat-panel.ts webapp/src/ui/styles/chat-panel.css webapp/tests/chat-panel.test.ts
git commit -m "feat(ui): Create ChatPanel component (with XSS protection)"
```

---

### Task 1.3: Create CodePanel Component

**Key security:** Use textContent for code display, wrap in pre/code safely.

```typescript
// webapp/src/ui/code-panel.ts
import { CodeOutput } from '../types/chat';

export class CodePanel {
  private container: HTMLElement;
  private code: CodeOutput | null = null;

  constructor(containerId: string = 'code-panel') {
    this.container = document.getElementById(containerId) || document.createElement('div');
  }

  render(): HTMLElement {
    const div = document.createElement('div');
    div.className = 'code-panel';

    if (!this.code?.code) {
      const p = document.createElement('p');
      p.textContent = 'No code generated yet';
      div.appendChild(p);
      return div;
    }

    // Header
    const header = document.createElement('div');
    header.className = 'code-header';

    const lang = document.createElement('span');
    lang.className = 'language';
    lang.textContent = this.code.language;

    const status = document.createElement('span');
    status.className = `status ${this.code.status}`;
    status.textContent = this.code.status;

    header.appendChild(lang);
    header.appendChild(status);
    div.appendChild(header);

    // Code block
    const pre = document.createElement('pre');
    const code = document.createElement('code');
    code.textContent = this.code.code; // SAFE: textContent
    pre.appendChild(code);
    div.appendChild(pre);

    // Output
    if (this.code.output) {
      const output = document.createElement('div');
      output.className = 'output';
      output.textContent = this.code.output;
      div.appendChild(output);
    }

    // Error
    if (this.code.error) {
      const error = document.createElement('div');
      error.className = 'error';
      error.textContent = this.code.error;
      div.appendChild(error);
    }

    return div;
  }

  setCode(code: string, language: string): void {
    this.code = { code, language, status: 'generated' };
    this.refresh();
  }

  setStatus(status: 'generated' | 'executing' | 'error'): void {
    if (this.code) {
      this.code.status = status;
      this.refresh();
    }
  }

  private refresh(): void {
    this.container.innerHTML = '';
    this.container.appendChild(this.render());
  }
}
```

**Commit:**

```bash
git add webapp/src/ui/code-panel.ts
git commit -m "feat(ui): Create CodePanel component (with XSS protection)"
```

---

### Task 1.4: Create LayoutManager

**Files:**
- Create: `webapp/src/ui/layout-manager.ts`
- Create: `webapp/src/ui/styles/layout.css`

```typescript
// webapp/src/ui/layout-manager.ts
import { ChatPanel } from './chat-panel';
import { CodePanel } from './code-panel';

export class LayoutManager {
  private chatPanel: ChatPanel;
  private codePanel: CodePanel;
  private container: HTMLElement;

  constructor() {
    this.chatPanel = new ChatPanel();
    this.codePanel = new CodePanel();
    this.container = document.createElement('div');
  }

  render(): HTMLElement {
    this.container.className = 'layout-container';

    const chatCol = document.createElement('div');
    chatCol.className = 'chat-column';
    chatCol.appendChild(this.chatPanel.render());

    const codeCol = document.createElement('div');
    codeCol.className = 'code-column';
    codeCol.appendChild(this.codePanel.render());

    this.container.appendChild(chatCol);
    this.container.appendChild(codeCol);

    return this.container;
  }

  getChatPanel(): ChatPanel {
    return this.chatPanel;
  }

  getCodePanel(): CodePanel {
    return this.codePanel;
  }
}
```

**CSS:**

```css
/* webapp/src/ui/styles/layout.css */
.layout-container {
  display: grid;
  grid-template-columns: 1fr 1fr;
  height: 100vh;
  gap: 0;
}

.chat-column {
  display: flex;
  flex-direction: column;
  border-right: 1px solid #ddd;
}

.code-column {
  display: flex;
  flex-direction: column;
  background-color: #fafafa;
}

.chat-panel, .code-panel {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

#chat-input {
  flex: 1;
  padding: 8px;
  border: 1px solid #ccc;
  border-radius: 4px;
}

#send-btn {
  padding: 8px 16px;
  background-color: #1976d2;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

pre {
  background-color: #263238;
  color: #aed581;
  padding: 12px;
  border-radius: 4px;
  overflow-x: auto;
}

code {
  font-family: 'Courier New', monospace;
  font-size: 12px;
}
```

**Commit:**

```bash
git add webapp/src/ui/layout-manager.ts webapp/src/ui/styles/layout.css
git commit -m "feat(ui): Create LayoutManager with split-screen layout"
```

---

## GROUP B: Backend API

### Task 2.1: Verify Chat & Learning Endpoints

**Files:**
- Create: `tests/api/test_endpoints.py`

```python
# tests/api/test_endpoints.py
import pytest
from fastapi.testclient import TestClient
from cynic.api.server import app

client = TestClient(app)

def test_chat_endpoint_exists():
    response = client.post("/api/chat/message", json={
        "text": "test",
        "session_id": "s1"
    })
    assert response.status_code != 404

def test_learn_endpoint_exists():
    response = client.post("/api/learn", json={
        "session_id": "s1",
        "prompt": "test",
        "code_generated": "def test(): pass",
        "user_feedback": "good"
    })
    assert response.status_code != 404
```

**Run:**

```bash
cd cynic && pytest tests/api/test_endpoints.py -v
```

**Commit:**

```bash
git add tests/api/test_endpoints.py
git commit -m "test(api): Verify chat and learning endpoints exist"
```

---

### Task 2.2: Create API Clients

**Files:**
- Create: `webapp/src/api/chat.ts`
- Create: `webapp/src/api/learning.ts`

```typescript
// webapp/src/api/chat.ts
export interface ChatRequest {
  text: string;
  session_id: string;
}

export interface ChatResponse {
  code: string;
  language: string;
  judgment_id: string;
}

export class ChatClient {
  private baseUrl: string = '/api';

  async sendMessage(req: ChatRequest): Promise<ChatResponse> {
    const response = await fetch(`${this.baseUrl}/chat/message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    if (!response.ok) throw new Error(`Chat error: ${response.status}`);
    return response.json();
  }
}

// webapp/src/api/learning.ts
export interface LearningRequest {
  session_id: string;
  prompt: string;
  code_generated: string;
  user_feedback: 'good' | 'needs_fix' | 'rejected';
}

export class LearningClient {
  private baseUrl: string = '/api';

  async submitEvent(req: LearningRequest): Promise<{ event_id: string }> {
    const response = await fetch(`${this.baseUrl}/learn`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    if (!response.ok) throw new Error(`Learning error: ${response.status}`);
    return response.json();
  }
}
```

**Commit:**

```bash
git add webapp/src/api/chat.ts webapp/src/api/learning.ts
git commit -m "feat(api): Create Chat and Learning API clients"
```

---

## GROUP C: Integration

### Task 3.1: Wire Everything Together in main.ts

**Files:**
- Modify: `webapp/src/main.ts`

```typescript
// webapp/src/main.ts
import { LayoutManager } from './ui/layout-manager';
import { ChatClient } from './api/chat';
import { LearningClient } from './api/learning';
import './ui/styles/layout.css';

async function initializeApp(): Promise<void> {
  const layoutManager = new LayoutManager();
  const chatClient = new ChatClient();
  const learningClient = new LearningClient();

  let sessionId = `session-${Date.now()}`;

  // Mount layout
  document.body.appendChild(layoutManager.render());

  const chatPanel = layoutManager.getChatPanel();
  const codePanel = layoutManager.getCodePanel();

  // Wire chat send
  chatPanel.onSend(async (text) => {
    // Add user message
    chatPanel.addMessage({
      id: `msg-${Date.now()}`,
      role: 'user',
      content: text,
      judgment_id: '',
      timestamp: Date.now(),
    });

    try {
      // Call API
      const response = await chatClient.sendMessage({ text, session_id: sessionId });

      // Display code
      codePanel.setCode(response.code, response.language);

      // Store judgment_id
      const judgmentId = response.judgment_id;

      // Add assistant message
      chatPanel.addMessage({
        id: `msg-${Date.now()}`,
        role: 'assistant',
        content: `Generated ${response.language} code`,
        code: response.code,
        judgment_id: judgmentId,
        timestamp: Date.now(),
      });

      // TODO: Add feedback buttons (Task 3.2)

    } catch (error) {
      codePanel.setStatus('error');
      console.error('Error:', error);
    }
  });

  console.log('*sniff* CYNIC Chat/Code initialized');
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initializeApp);
} else {
  initializeApp();
}
```

**Commit:**

```bash
git add webapp/src/main.ts
git commit -m "feat(integration): Wire ChatPanel to API and display code"
```

---

### Task 3.2: Add Learning Event Storage

**Files:**
- Create: `cynic/cynic/storage/learning_events.py`

```python
# cynic/cynic/storage/learning_events.py
import asyncpg
import uuid
from datetime import datetime

class LearningEventStorage:
    def __init__(self, pool: asyncpg.Pool):
        self.pool = pool

    async def init_tables(self):
        async with self.pool.acquire() as conn:
            await conn.execute("""
                CREATE TABLE IF NOT EXISTS learning_events (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    prompt TEXT,
                    code_generated TEXT,
                    user_feedback TEXT,
                    timestamp BIGINT,
                    senior_dev_status TEXT DEFAULT 'pending'
                )
            """)

    async def store_event(self, session_id: str, prompt: str, code: str, feedback: str):
        event_id = str(uuid.uuid4())
        async with self.pool.acquire() as conn:
            await conn.execute("""
                INSERT INTO learning_events
                (id, session_id, prompt, code_generated, user_feedback, timestamp)
                VALUES ($1, $2, $3, $4, $5, $6)
            """, event_id, session_id, prompt, code, feedback, int(datetime.now().timestamp()))
        return event_id
```

**Commit:**

```bash
git add cynic/cynic/storage/learning_events.py
git commit -m "feat(storage): Add learning event storage to database"
```

---

## GROUP D: Testing & Docker

### Task 4.1: E2E Test

**Files:**
- Create: `tests/e2e/test_phase1_flow.py`

```python
# tests/e2e/test_phase1_flow.py
def test_full_flow():
    """Chat → Code → Learning Event"""
    client = TestClient(app)

    # 1. Chat
    r1 = client.post("/api/chat/message", json={"text": "test", "session_id": "e2e"})
    assert r1.status_code == 200
    code = r1.json()['code']

    # 2. Learn
    r2 = client.post("/api/learn", json={
        "session_id": "e2e",
        "prompt": "test",
        "code_generated": code,
        "user_feedback": "good"
    })
    assert r2.status_code == 200
```

**Commit:**

```bash
git add tests/e2e/test_phase1_flow.py
git commit -m "test(e2e): Add Phase 1 full flow test"
```

---

### Task 4.2: Docker Verification

**Run:**

```bash
docker build -t cynic:phase1 .
docker run -p 8000:8000 cynic:phase1

# In another terminal:
curl http://localhost:8000/
curl -X POST http://localhost:8000/api/chat/message \
  -H "Content-Type: application/json" \
  -d '{"text":"test","session_id":"s1"}'
```

**Success = HTTP 200 responses**

---

## Summary

**Total Tasks:** 10
**Estimated LOC:** ~900
**Tests:** 15+
**Parallel:** Group A tasks can run simultaneously
**Success Criteria:** All tests pass + Docker works

---

## Execution

**Two options:**

**Option 1: Subagent-Driven (Recommended)** ⭐
- Use `superpowers:subagent-driven-development`
- Fresh agent per task
- Code review between tasks
- **Recommended for this complex project**

**Option 2: Manual**
- Follow tasks in current session
- Self-review

**Which one?**
