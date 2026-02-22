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
