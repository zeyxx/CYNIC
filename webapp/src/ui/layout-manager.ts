import { ChatPanel } from './chat-panel';
import { CodePanel } from './code-panel';

export class LayoutManager {
  private chatPanel: ChatPanel;
  private codePanel: CodePanel;
  private container: HTMLElement;

  constructor() {
    this.chatPanel = new ChatPanel();
    this.codePanel = new CodePanel();
    this.container = document.createElement('div');
  }

  render(): HTMLElement {
    this.container.className = 'layout-container';

    const chatCol = document.createElement('div');
    chatCol.className = 'chat-column';
    chatCol.appendChild(this.chatPanel.render());

    const codeCol = document.createElement('div');
    codeCol.className = 'code-column';
    codeCol.appendChild(this.codePanel.render());

    this.container.appendChild(chatCol);
    this.container.appendChild(codeCol);

    return this.container;
  }

  getChatPanel(): ChatPanel {
    return this.chatPanel;
  }

  getCodePanel(): CodePanel {
    return this.codePanel;
  }
}
