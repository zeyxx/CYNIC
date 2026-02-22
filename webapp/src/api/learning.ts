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
