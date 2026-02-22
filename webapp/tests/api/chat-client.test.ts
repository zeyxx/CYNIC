import { describe, it, expect } from 'vitest';
import { ChatClient, ChatRequest, ChatResponse } from '../../src/api/chat';
import { LearningClient, LearningRequest } from '../../src/api/learning';

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

  it('should type ChatResponse correctly', () => {
    const response: ChatResponse = {
      code: 'const x = 1;',
      language: 'typescript',
      judgment_id: 'j-123',
    };
    expect(response.code).toBe('const x = 1;');
    expect(response.language).toBe('typescript');
    expect(response.judgment_id).toBe('j-123');
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
});
