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
