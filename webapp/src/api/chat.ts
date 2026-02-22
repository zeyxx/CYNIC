import { fetchWithTimeout, ValidationError } from './shared';

export interface ChatRequest {
  text: string;
  session_id: string;
}

export interface ChatResponse {
  text: string;
  session_id: string;
}

export class ChatClient {
  private baseUrl: string = '/api';

  async sendMessage(req: ChatRequest): Promise<ChatResponse> {
    if (!req.text || !req.session_id) {
      throw new ValidationError('ChatRequest requires text and session_id');
    }

    const response = await fetchWithTimeout<ChatResponse>(
      `${this.baseUrl}/chat/message`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(req),
      },
      5000
    );

    return response;
  }
}
