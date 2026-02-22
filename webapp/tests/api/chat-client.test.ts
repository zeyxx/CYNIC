import { describe, it, expect } from 'vitest';
import { ChatClient, ChatRequest, ChatResponse } from '../../src/api/chat';
import { LearningClient, LearningRequest, LearningResponse } from '../../src/api/learning';
import { NetworkError, ValidationError, ServerError } from '../../src/api/shared';

describe('ChatClient', () => {
  it('should construct ChatRequest with correct types', () => {
    const req: ChatRequest = {
      text: 'hello world',
      session_id: 'session-123',
    };
    expect(req.text).toBe('hello world');
    expect(req.session_id).toBe('session-123');
  });

  it('should have ChatClient class with sendMessage method', () => {
    const client = new ChatClient();
    expect(client).toBeDefined();
    expect(typeof client.sendMessage).toBe('function');
  });

  it('should type ChatResponse correctly with fixed interface', () => {
    const response: ChatResponse = {
      text: 'hello response',
      session_id: 'session-123',
    };
    expect(response.text).toBe('hello response');
    expect(response.session_id).toBe('session-123');
  });

  it('should throw ValidationError for missing request fields', async () => {
    const client = new ChatClient();
    const invalidReq = { text: '', session_id: '' } as ChatRequest;

    try {
      await client.sendMessage(invalidReq);
      expect.fail('should have thrown ValidationError');
    } catch (e) {
      expect(e).toBeInstanceOf(ValidationError);
    }
  });
});

describe('LearningClient', () => {
  it('should construct LearningRequest with correct types', () => {
    const req: LearningRequest = {
      session_id: 'session-123',
      prompt: 'write a function',
      code_generated: 'function test() {}',
      user_feedback: 'good',
    };
    expect(req.session_id).toBe('session-123');
    expect(req.prompt).toBe('write a function');
    expect(req.code_generated).toBe('function test() {}');
    expect(req.user_feedback).toBe('good');
  });

  it('should accept all feedback types', () => {
    const goodReq: LearningRequest = {
      session_id: 's1',
      prompt: 'test',
      code_generated: 'code',
      user_feedback: 'good',
    };
    const fixReq: LearningRequest = {
      session_id: 's1',
      prompt: 'test',
      code_generated: 'code',
      user_feedback: 'needs_fix',
    };
    const rejReq: LearningRequest = {
      session_id: 's1',
      prompt: 'test',
      code_generated: 'code',
      user_feedback: 'rejected',
    };
    expect(goodReq.user_feedback).toBe('good');
    expect(fixReq.user_feedback).toBe('needs_fix');
    expect(rejReq.user_feedback).toBe('rejected');
  });

  it('should have LearningClient class with submitEvent method', () => {
    const client = new LearningClient();
    expect(client).toBeDefined();
    expect(typeof client.submitEvent).toBe('function');
  });

  it('should type LearningResponse correctly', () => {
    const response: LearningResponse = {
      event_id: 'event-123',
    };
    expect(response.event_id).toBe('event-123');
  });

  it('should throw ValidationError for invalid user_feedback', async () => {
    const client = new LearningClient();
    const invalidReq = {
      session_id: 's1',
      prompt: 'test',
      code_generated: 'code',
      user_feedback: 'invalid',
    } as any as LearningRequest;

    try {
      await client.submitEvent(invalidReq);
      expect.fail('should have thrown ValidationError');
    } catch (e) {
      expect(e).toBeInstanceOf(ValidationError);
    }
  });

  it('should throw ValidationError for missing request fields', async () => {
    const client = new LearningClient();
    const invalidReq = {
      session_id: '',
      prompt: '',
      code_generated: '',
      user_feedback: 'good',
    } as any as LearningRequest;

    try {
      await client.submitEvent(invalidReq);
      expect.fail('should have thrown ValidationError');
    } catch (e) {
      expect(e).toBeInstanceOf(ValidationError);
    }
  });
});

describe('Error Classes', () => {
  it('should create NetworkError instances', () => {
    const err = new NetworkError('Connection failed', 503);
    expect(err.name).toBe('NetworkError');
    expect(err.message).toBe('Connection failed');
    expect(err.status).toBe(503);
  });

  it('should create ValidationError instances', () => {
    const err = new ValidationError('Invalid input');
    expect(err.name).toBe('ValidationError');
    expect(err.message).toBe('Invalid input');
  });

  it('should create ServerError instances', () => {
    const err = new ServerError('Server error', 500);
    expect(err.name).toBe('ServerError');
    expect(err.message).toBe('Server error');
    expect(err.status).toBe(500);
  });
});
