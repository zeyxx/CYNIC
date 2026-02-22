import { fetchWithTimeout, ValidationError } from './shared';

export interface LearningRequest {
  session_id: string;
  prompt: string;
  code_generated: string;
  user_feedback: 'good' | 'needs_fix' | 'rejected';
}

export interface LearningResponse {
  event_id: string;
}

export class LearningClient {
  private baseUrl: string = '/api';

  async submitEvent(req: LearningRequest): Promise<LearningResponse> {
    if (!req.session_id || !req.prompt || !req.code_generated) {
      throw new ValidationError('LearningRequest requires session_id, prompt, and code_generated');
    }

    if (!['good', 'needs_fix', 'rejected'].includes(req.user_feedback)) {
      throw new ValidationError(
        `Invalid user_feedback: "${req.user_feedback}". Must be one of: good, needs_fix, rejected`
      );
    }

    const response = await fetchWithTimeout<LearningResponse>(
      `${this.baseUrl}/learn`,
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
