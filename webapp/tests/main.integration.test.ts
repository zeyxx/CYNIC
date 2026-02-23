import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { initializeApp } from '../src/main';
import { ChatClient, ChatResponse } from '../src/api/chat';

describe('main.ts Integration', () => {
  let sendMessageSpy: any;

  beforeEach(() => {
    // Mock ChatClient.sendMessage before app initialization
    sendMessageSpy = vi.spyOn(ChatClient.prototype, 'sendMessage').mockResolvedValue({
      text: 'Generated code for you',
      session_id: 'session-123',
      code: 'def hello():\n  print("CYNIC")',
      language: 'python',
      judgment_id: 'j-123',
    } as ChatResponse);
  });

  afterEach(() => {
    vi.restoreAllMocks();

    // Clean up DOM
    const layout = document.querySelector('.layout-container');
    if (layout?.parentElement) {
      layout.parentElement.removeChild(layout);
    }
  });

  it('should initialize app and mount layout to DOM', async () => {
    await initializeApp();

    const layout = document.querySelector('.layout-container');
    expect(layout).toBeTruthy();
  });

  it('should mount chat and code panels within layout', async () => {
    await initializeApp();

    const chatPanel = document.querySelector('.chat-panel');
    const codePanel = document.querySelector('.code-panel');

    expect(chatPanel).toBeTruthy();
    expect(codePanel).toBeTruthy();
  });

  it('should wire chat send event and call ChatClient with correct session', async () => {
    await initializeApp();

    const chatInput = document.getElementById('chat-input') as HTMLInputElement;
    const sendBtn = document.getElementById('send-btn') as HTMLButtonElement;

    expect(chatInput).toBeTruthy();
    expect(sendBtn).toBeTruthy();

    // Simulate user typing and sending
    chatInput.value = 'Create a hello function';
    sendBtn.click();

    // Wait for async operations
    await new Promise(resolve => setTimeout(resolve, 250));

    // Verify ChatClient was called with correct parameters
    expect(sendMessageSpy).toHaveBeenCalled();
    const call = sendMessageSpy.mock.calls[0];
    expect(call[0].text).toBe('Create a hello function');
    expect(call[0].session_id).toMatch(/^session-\d+$/);
  });

  it('should call ChatClient for each user message', async () => {
    await initializeApp();

    const chatInput = document.getElementById('chat-input') as HTMLInputElement;
    const sendBtn = document.getElementById('send-btn') as HTMLButtonElement;

    // Send first message
    chatInput.value = 'message 1';
    sendBtn.click();

    await new Promise(resolve => setTimeout(resolve, 150));

    expect(sendMessageSpy).toHaveBeenCalledTimes(1);
    expect(sendMessageSpy.mock.calls[0][0].text).toBe('message 1');

    // Send second message
    chatInput.value = 'message 2';
    sendBtn.click();

    await new Promise(resolve => setTimeout(resolve, 150));

    expect(sendMessageSpy).toHaveBeenCalledTimes(2);
    expect(sendMessageSpy.mock.calls[1][0].text).toBe('message 2');
  });

  it('should persist session ID across multiple messages', async () => {
    await initializeApp();

    const chatInput = document.getElementById('chat-input') as HTMLInputElement;
    const sendBtn = document.getElementById('send-btn') as HTMLButtonElement;

    // Send first message
    chatInput.value = 'message 1';
    sendBtn.click();

    await new Promise(resolve => setTimeout(resolve, 150));

    const sessionId1 = sendMessageSpy.mock.calls[0][0].session_id;

    // Send second message
    chatInput.value = 'message 2';
    sendBtn.click();

    await new Promise(resolve => setTimeout(resolve, 150));

    const sessionId2 = sendMessageSpy.mock.calls[1][0].session_id;

    // Both should use the same session ID
    expect(sessionId1).toBe(sessionId2);
    expect(sessionId1).toMatch(/^session-\d+$/);
  });

  it('should call ChatClient even when errors occur', async () => {
    sendMessageSpy.mockRejectedValueOnce(new Error('Network error: Failed to fetch'));

    await initializeApp();

    const chatInput = document.getElementById('chat-input') as HTMLInputElement;
    const sendBtn = document.getElementById('send-btn') as HTMLButtonElement;

    chatInput.value = 'test prompt';
    sendBtn.click();

    await new Promise(resolve => setTimeout(resolve, 200));

    // Verify ChatClient was still called despite error
    expect(sendMessageSpy).toHaveBeenCalled();
    expect(sendMessageSpy.mock.calls[0][0].text).toBe('test prompt');
  });
});
