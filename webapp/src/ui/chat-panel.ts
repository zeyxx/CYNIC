import { ChatMessage } from '../types/chat';

export class ChatPanel {
  private container: HTMLElement;
  private messages: ChatMessage[] = [];
  private sendCallback?: (text: string) => void;

  constructor(containerId: string = 'chat-panel') {
    this.container = document.getElementById(containerId) || document.createElement('div');
  }

  render(): HTMLElement {
    const div = document.createElement('div');
    div.className = 'chat-panel';

    // Messages container
    const msgDiv = document.createElement('div');
    msgDiv.className = 'messages';

    this.messages.forEach(msg => {
      const msgEl = document.createElement('div');
      msgEl.className = `message ${msg.role}`;

      const contentEl = document.createElement('div');
      contentEl.className = 'message-content';
      contentEl.textContent = msg.content; // SAFE: textContent prevents XSS

      const timeEl = document.createElement('div');
      timeEl.className = 'timestamp';
      timeEl.textContent = new Date(msg.timestamp).toLocaleTimeString();

      msgEl.appendChild(contentEl);
      msgEl.appendChild(timeEl);
      msgDiv.appendChild(msgEl);
    });

    div.appendChild(msgDiv);

    // Input area
    const inputArea = document.createElement('div');
    inputArea.className = 'input-area';

    const input = document.createElement('input');
    input.type = 'text';
    input.id = 'chat-input';
    input.placeholder = 'Type your intent...';

    const sendBtn = document.createElement('button');
    sendBtn.id = 'send-btn';
    sendBtn.textContent = 'Send';
    sendBtn.addEventListener('click', () => {
      const text = input.value.trim();
      if (text && this.sendCallback) {
        this.sendCallback(text);
        input.value = '';
      }
    });

    inputArea.appendChild(input);
    inputArea.appendChild(sendBtn);
    div.appendChild(inputArea);

    return div;
  }

  addMessage(msg: ChatMessage): void {
    this.messages.push(msg);
    this.refresh();
  }

  onSend(callback: (text: string) => void): void {
    this.sendCallback = callback;
  }

  private refresh(): void {
    // Clear and re-render: empty string is safe, followed by legitimate appendChild
    this.container.innerHTML = '';
    this.container.appendChild(this.render());
  }
}
