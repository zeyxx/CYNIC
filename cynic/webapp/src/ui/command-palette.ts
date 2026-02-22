import { createFormFromSchema } from './form-builder';
import type { OrganismSchema, CommandSchema } from '../types/api';

export class CommandPalette {
  private containerSelector: string;
  private schema: OrganismSchema;
  private modal: HTMLElement | null = null;
  private backdrop: HTMLElement | null = null;
  private searchInput: HTMLInputElement | null = null;
  private commandList: HTMLElement | null = null;
  private formContainer: HTMLElement | null = null;
  private selectedCommand: CommandSchema | null = null;
  private debounceTimer: ReturnType<typeof setTimeout> | null = null;
  private debounceDelay = 200;
  private escapeKeyHandler: ((e: KeyboardEvent) => void) | null = null;
  private backdropClickHandler: ((e: MouseEvent) => void) | null = null;
  private formSubmitHandler: ((e: Event) => void) | null = null;

  constructor(containerSelector: string, schema: OrganismSchema) {
    this.containerSelector = containerSelector;
    this.schema = schema;
  }

  /**
   * Open the palette modal
   */
  public open(): void {
    if (this.modal !== null) {
      return; // Already open
    }

    this.createModal();
    this.attachEventListeners();
  }

  /**
   * Close the palette modal
   */
  public close(): void {
    this.detachEventListeners();
    this.removeModal();
  }

  /**
   * Render the modal structure
   */
  private createModal(): void {
    const container = document.querySelector(this.containerSelector);
    if (!container) {
      throw new Error(`Container ${this.containerSelector} not found`);
    }

    // Create backdrop
    this.backdrop = document.createElement('div');
    this.backdrop.className = 'command-palette-backdrop';

    // Create modal
    this.modal = document.createElement('div');
    this.modal.className = 'command-palette-modal';

    // Create search input
    this.searchInput = document.createElement('input');
    this.searchInput.type = 'text';
    this.searchInput.className = 'command-palette-search';
    this.searchInput.placeholder = 'Search commands...';
    this.modal.appendChild(this.searchInput);

    // Create command list container
    this.commandList = document.createElement('div');
    this.commandList.className = 'command-palette-list';
    this.modal.appendChild(this.commandList);

    // Create form container (hidden by default)
    this.formContainer = document.createElement('div');
    this.formContainer.className = 'command-palette-form-container';
    this.formContainer.style.display = 'none';
    this.modal.appendChild(this.formContainer);

    // Render initial command list
    this.renderCommandList(this.schema.commands);

    // Append to container
    container.appendChild(this.backdrop);
    container.appendChild(this.modal);
  }

  /**
   * Remove the modal from DOM
   */
  private removeModal(): void {
    if (this.backdrop) {
      this.backdrop.remove();
      this.backdrop = null;
    }
    if (this.modal) {
      this.modal.remove();
      this.modal = null;
    }
    this.searchInput = null;
    this.commandList = null;
    this.formContainer = null;
    this.selectedCommand = null;
  }

  /**
   * Render the command list using safe DOM methods (no innerHTML)
   */
  private renderCommandList(commands: CommandSchema[]): void {
    if (!this.commandList) return;

    // Clear existing children
    while (this.commandList.firstChild) {
      this.commandList.removeChild(this.commandList.firstChild);
    }

    if (commands.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'command-palette-empty';
      empty.textContent = 'No commands found';
      this.commandList.appendChild(empty);
      return;
    }

    // Create items using safe DOM methods
    for (const cmd of commands) {
      const item = document.createElement('div');
      item.className = 'command-item';
      item.setAttribute('data-command-id', cmd.id);

      const nameEl = document.createElement('strong');
      nameEl.textContent = cmd.name;

      const descEl = document.createElement('p');
      descEl.textContent = cmd.description;

      item.appendChild(nameEl);
      item.appendChild(descEl);

      this.commandList.appendChild(item);
    }
  }

  /**
   * Filter commands based on search query
   */
  private handleSearch(query: string): void {
    const filtered = this.schema.commands.filter((cmd) => {
      const lowerQuery = query.toLowerCase();
      return cmd.name.toLowerCase().includes(lowerQuery) || cmd.description.toLowerCase().includes(lowerQuery);
    });

    this.renderCommandList(filtered);
  }

  /**
   * Handle command selection
   */
  private handleSelectCommand(commandId: string): void {
    const command = this.schema.commands.find((c) => c.id === commandId);
    if (!command) return;

    this.selectedCommand = command;

    // Hide command list
    if (this.commandList) {
      this.commandList.style.display = 'none';
    }

    // Show form container
    if (this.formContainer) {
      this.formContainer.style.display = 'block';

      // Clear existing form
      while (this.formContainer.firstChild) {
        this.formContainer.removeChild(this.formContainer.firstChild);
      }

      const form = createFormFromSchema(command);
      this.formContainer.appendChild(form);

      // Attach form submit handler
      this.attachFormSubmitHandler(form);

      // Focus on first input
      const firstInput = form.querySelector('input, select, textarea') as HTMLElement;
      if (firstInput) {
        firstInput.focus();
      }
    }
  }

  /**
   * Handle form submission
   */
  private attachFormSubmitHandler(form: HTMLFormElement): void {
    this.formSubmitHandler = (e: Event) => {
      e.preventDefault();

      // Reset UI to command list
      if (this.commandList) {
        this.commandList.style.display = 'block';
      }
      if (this.formContainer) {
        this.formContainer.style.display = 'none';

        // Clear form
        while (this.formContainer.firstChild) {
          this.formContainer.removeChild(this.formContainer.firstChild);
        }
      }
      this.selectedCommand = null;

      // Reset search
      if (this.searchInput) {
        this.searchInput.value = '';
        this.renderCommandList(this.schema.commands);
      }
    };

    form.addEventListener('submit', this.formSubmitHandler);
  }

  /**
   * Attach event listeners
   */
  private attachEventListeners(): void {
    // Search input handler with debounce
    if (this.searchInput) {
      this.searchInput.addEventListener('input', (e) => {
        const query = (e.target as HTMLInputElement).value;

        if (this.debounceTimer) {
          clearTimeout(this.debounceTimer);
        }

        this.debounceTimer = setTimeout(() => {
          this.handleSearch(query);
        }, this.debounceDelay);
      });
    }

    // Command list click handler (event delegation)
    if (this.commandList) {
      this.commandList.addEventListener('click', (e) => {
        const item = (e.target as HTMLElement).closest('.command-item');
        if (item) {
          const commandId = item.getAttribute('data-command-id');
          if (commandId) {
            this.handleSelectCommand(commandId);
          }
        }
      });
    }

    // Escape key handler
    this.escapeKeyHandler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        this.close();
      }
    };
    document.addEventListener('keydown', this.escapeKeyHandler);

    // Backdrop click handler
    this.backdropClickHandler = (e: MouseEvent) => {
      if (e.target === this.backdrop) {
        this.close();
      }
    };
    if (this.backdrop) {
      this.backdrop.addEventListener('click', this.backdropClickHandler);
    }
  }

  /**
   * Detach event listeners
   */
  private detachEventListeners(): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = null;
    }

    if (this.escapeKeyHandler) {
      document.removeEventListener('keydown', this.escapeKeyHandler);
      this.escapeKeyHandler = null;
    }

    if (this.backdrop && this.backdropClickHandler) {
      this.backdrop.removeEventListener('click', this.backdropClickHandler);
      this.backdropClickHandler = null;
    }

    if (this.formContainer) {
      const form = this.formContainer.querySelector('form');
      if (form && this.formSubmitHandler) {
        form.removeEventListener('submit', this.formSubmitHandler);
        this.formSubmitHandler = null;
      }
    }
  }
}
