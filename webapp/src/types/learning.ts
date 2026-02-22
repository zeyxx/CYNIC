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
