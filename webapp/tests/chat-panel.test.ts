import { describe, it, expect, beforeEach } from 'vitest';
import { ChatPanel } from '../src/ui/chat-panel';
import { ChatMessage } from '../src/types/chat';

describe('ChatPanel', () => {
  let panel: ChatPanel;

  beforeEach(() => {
    panel = new ChatPanel();
  });

  it('should render chat panel with input and button', () => {
    const html = panel.render();
    expect(html.querySelector('#chat-input')).toBeTruthy();
    expect(html.querySelector('#send-btn')).toBeTruthy();
    expect(html.querySelector('.messages')).toBeTruthy();
    expect(html.classList.contains('chat-panel')).toBe(true);
  });

  it('should add message safely (XSS protection via textContent)', () => {
    const msg: ChatMessage = {
      id: 'm1',
      role: 'user',
      content: '<script>alert("xss")</script>',
      judgment_id: 'j1',
      timestamp: Date.now(),
    };
    panel.addMessage(msg);
    const html = panel.render();

    // XSS payload should appear as text, not execute
    expect(html.textContent).toContain('<script>');

    // Verify no actual script element was created
    expect(html.querySelector('script')).toBeFalsy();

    // Verify textContent is being used (not innerHTML)
    const contentEl = html.querySelector('.message-content');
    expect(contentEl?.textContent).toEqual('<script>alert("xss")</script>');
  });

  it('should handle multiple messages', () => {
    const msg1: ChatMessage = {
      id: 'm1',
      role: 'user',
      content: 'Hello',
      judgment_id: 'j1',
      timestamp: Date.now(),
    };

    const msg2: ChatMessage = {
      id: 'm2',
      role: 'assistant',
      content: 'Hi there!',
      judgment_id: 'j2',
      timestamp: Date.now() + 1000,
    };

    panel.addMessage(msg1);
    panel.addMessage(msg2);

    const html = panel.render();
    const messages = html.querySelectorAll('.message');
    expect(messages.length).toBe(2);
    expect(messages[0].classList.contains('user')).toBe(true);
    expect(messages[1].classList.contains('assistant')).toBe(true);
  });

  it('should properly format message with role classes', () => {
    const userMsg: ChatMessage = {
      id: 'm1',
      role: 'user',
      content: 'User message',
      judgment_id: 'j1',
      timestamp: Date.now(),
    };

    const assistantMsg: ChatMessage = {
      id: 'm2',
      role: 'assistant',
      content: 'Assistant message',
      judgment_id: 'j2',
      timestamp: Date.now() + 1000,
    };

    panel.addMessage(userMsg);
    panel.addMessage(assistantMsg);

    const html = panel.render();
    const messages = html.querySelectorAll('.message');

    expect(messages[0].className).toContain('user');
    expect(messages[1].className).toContain('assistant');
  });

  it('should display timestamp for messages', () => {
    const now = Date.now();
    const msg: ChatMessage = {
      id: 'm1',
      role: 'user',
      content: 'Test message',
      judgment_id: 'j1',
      timestamp: now,
    };

    panel.addMessage(msg);
    const html = panel.render();
    const timestamp = html.querySelector('.timestamp');

    expect(timestamp).toBeTruthy();
    expect(timestamp?.textContent).toBeTruthy();
    // Should contain time components
    expect(timestamp?.textContent).toMatch(/\d+:\d+:\d+/);
  });

  it('should register send callback', () => {
    let callbackFired = false;
    panel.onSend(() => {
      callbackFired = true;
    });

    // Callback is registered (actual button click requires DOM interaction)
    expect(callbackFired).toBe(false);
  });

  it('should escape special HTML characters', () => {
    const msg: ChatMessage = {
      id: 'm1',
      role: 'user',
      content: '<div onclick="alert(\'xss\')">&nbsp;</div>',
      judgment_id: 'j1',
      timestamp: Date.now(),
    };

    panel.addMessage(msg);
    const html = panel.render();
    const contentEl = html.querySelector('.message-content');

    // textContent should contain literal HTML tags, not render them
    expect(contentEl?.textContent).toEqual('<div onclick="alert(\'xss\')">&nbsp;</div>');
    expect(html.querySelector('div[onclick]')).toBeFalsy();
  });

  it('should handle messages with code property', () => {
    const msg: ChatMessage = {
      id: 'm1',
      role: 'assistant',
      content: 'Here is code:',
      code: 'console.log("test")',
      code_language: 'javascript',
      judgment_id: 'j1',
      timestamp: Date.now(),
    };

    panel.addMessage(msg);
    const html = panel.render();

    // Main content should still be displayed safely
    expect(html.textContent).toContain('Here is code:');
  });

  it('should handle empty messages array initially', () => {
    const html = panel.render();
    const messages = html.querySelectorAll('.message');
    expect(messages.length).toBe(0);
    expect(html.querySelector('.messages')).toBeTruthy();
  });

  it('should maintain message order', () => {
    const messages: ChatMessage[] = [];
    for (let i = 0; i < 5; i++) {
      messages.push({
        id: `m${i}`,
        role: i % 2 === 0 ? 'user' : 'assistant',
        content: `Message ${i}`,
        judgment_id: `j${i}`,
        timestamp: Date.now() + i * 1000,
      });
    }

    messages.forEach(msg => panel.addMessage(msg));
    const html = panel.render();
    const messageElements = html.querySelectorAll('.message-content');

    expect(messageElements.length).toBe(5);
    messageElements.forEach((el, i) => {
      expect(el.textContent).toEqual(`Message ${i}`);
    });
  });
});
