import { CodeOutput } from '../types/chat';

export class CodePanel {
  private container: HTMLElement;
  private code: CodeOutput | null = null;

  constructor(containerId: string = 'code-panel') {
    this.container = document.getElementById(containerId) || document.createElement('div');
  }

  render(): HTMLElement {
    const div = document.createElement('div');
    div.className = 'code-panel';

    if (!this.code?.code) {
      const p = document.createElement('p');
      p.textContent = 'No code generated yet';
      div.appendChild(p);
      return div;
    }

    // Header
    const header = document.createElement('div');
    header.className = 'code-header';

    const lang = document.createElement('span');
    lang.className = 'language';
    lang.textContent = this.code.language;

    const status = document.createElement('span');
    status.className = `status ${this.code.status}`;
    status.textContent = this.code.status;

    header.appendChild(lang);
    header.appendChild(status);
    div.appendChild(header);

    // Code block
    const pre = document.createElement('pre');
    const code = document.createElement('code');
    code.textContent = this.code.code; // SAFE: textContent
    pre.appendChild(code);
    div.appendChild(pre);

    // Output
    if (this.code.output) {
      const output = document.createElement('div');
      output.className = 'output';
      output.textContent = this.code.output;
      div.appendChild(output);
    }

    // Error
    if (this.code.error) {
      const error = document.createElement('div');
      error.className = 'error';
      error.textContent = this.code.error;
      div.appendChild(error);
    }

    return div;
  }

  setCode(code: string, language: string): void {
    this.code = { code, language, status: 'generated' };
    this.refresh();
  }

  setStatus(status: 'generated' | 'executing' | 'error'): void {
    if (this.code) {
      this.code.status = status;
      this.refresh();
    }
  }

  setOutput(output: string): void {
    if (this.code) {
      this.code.output = output;
      this.refresh();
    }
  }

  setError(error: string): void {
    if (this.code) {
      this.code.error = error;
      this.code.status = 'error';
      this.refresh();
    }
  }

  private refresh(): void {
    // Safe DOM manipulation: remove all children and append new content
    while (this.container.firstChild) {
      this.container.removeChild(this.container.firstChild);
    }
    this.container.appendChild(this.render());
  }
}
