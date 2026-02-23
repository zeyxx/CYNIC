import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { CommandPalette } from '../src/ui/command-palette';
import type { OrganismSchema, CommandSchema } from '../src/types/api';

const MOCK_SCHEMA: OrganismSchema = {
  version: '1.0.0',
  commands: [
    {
      id: 'get-status',
      name: 'Get Status',
      description: 'Fetch organism status and health metrics',
      params: {
        verbose: { type: 'boolean', required: false, description: 'Include extra detail' },
      },
      returns: { type: 'object', description: 'Status object' },
    },
    {
      id: 'learn-rate',
      name: 'Update Learn Rate',
      description: 'Adjust the learning rate parameter',
      params: {
        rate: { type: 'number', required: true, description: 'New learning rate (0-1)' },
      },
      returns: { type: 'object', description: 'Updated parameters' },
    },
    {
      id: 'confidence-score',
      name: 'Confidence Score',
      description: 'Get current confidence metrics',
      params: {},
      returns: { type: 'object', description: 'Confidence breakdown' },
    },
  ],
  skills: [],
  state: {},
};

describe('CommandPalette', () => {
  let container: HTMLDivElement;
  let palette: CommandPalette;

  beforeEach(() => {
    // Create container
    container = document.createElement('div');
    container.id = 'palette-container';
    document.body.appendChild(container);
  });

  afterEach(() => {
    // Clean up
    palette?.close();
    document.body.removeChild(container);
    vi.clearAllMocks();
  });

  // Test 1: Modal creation and rendering
  it('creates and renders modal element on open()', () => {
    palette = new CommandPalette('#palette-container', MOCK_SCHEMA);
    palette.open();

    const modal = document.querySelector('.command-palette-modal');
    expect(modal).toBeTruthy();

    const backdrop = document.querySelector('.command-palette-backdrop');
    expect(backdrop).toBeTruthy();

    const searchInput = modal?.querySelector('.command-palette-search') as HTMLInputElement;
    expect(searchInput).toBeTruthy();
    expect(searchInput.type).toBe('text');
    expect(searchInput.placeholder).toBe('Search commands...');

    const commandList = modal?.querySelector('.command-palette-list');
    expect(commandList).toBeTruthy();
  });

  // Test 2: Command filtering by search query
  it('filters commands based on search query (case-insensitive)', async () => {
    palette = new CommandPalette('#palette-container', MOCK_SCHEMA);
    palette.open();

    const searchInput = document.querySelector('.command-palette-search') as HTMLInputElement;
    const commandList = document.querySelector('.command-palette-list') as HTMLElement;

    // Search for "status"
    searchInput.value = 'status';
    searchInput.dispatchEvent(new Event('input', { bubbles: true }));

    // Wait for debounce (200ms default)
    await new Promise((resolve) => setTimeout(resolve, 250));

    const items = commandList.querySelectorAll('.command-item');
    expect(items.length).toBe(1);
    expect(items[0].textContent).toContain('Get Status');
  });

  // Test 3: Form injection when command selected
  it('injects form when command is selected', async () => {
    palette = new CommandPalette('#palette-container', MOCK_SCHEMA);
    palette.open();

    const commandList = document.querySelector('.command-palette-list') as HTMLElement;
    const firstItem = commandList.querySelector('.command-item') as HTMLElement;

    firstItem.click();

    // Form should appear in modal
    const modal = document.querySelector('.command-palette-modal');
    const form = modal?.querySelector('form');
    expect(form).toBeTruthy();
    expect(form?.getAttribute('data-command')).toBe('Get Status');

    // Command list should be hidden or replaced
    const listAfterSelect = modal?.querySelector('.command-palette-list');
    expect(listAfterSelect?.style.display).toBe('none');
  });

  // Test 4: Close on escape key
  it('closes palette when Escape key is pressed', () => {
    palette = new CommandPalette('#palette-container', MOCK_SCHEMA);
    palette.open();

    let modal = document.querySelector('.command-palette-modal');
    expect(modal).toBeTruthy();

    const event = new KeyboardEvent('keydown', { key: 'Escape' });
    document.dispatchEvent(event);

    modal = document.querySelector('.command-palette-modal');
    expect(modal).toBeFalsy();
  });

  // Test 5: Search performance (no memory leak with large command list)
  it('handles large command list without memory leaks', async () => {
    const largeSchema: OrganismSchema = {
      ...MOCK_SCHEMA,
      commands: Array.from({ length: 500 }, (_, i) => ({
        id: `cmd-${i}`,
        name: `Command ${i}`,
        description: `Description for command ${i}`,
        params: {},
        returns: { type: 'object', description: 'Result' },
      })),
    };

    palette = new CommandPalette('#palette-container', largeSchema);
    palette.open();

    const searchInput = document.querySelector('.command-palette-search') as HTMLInputElement;

    // Perform multiple searches
    for (let i = 0; i < 20; i++) {
      searchInput.value = `Command ${i}`;
      searchInput.dispatchEvent(new Event('input', { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 50));
    }

    palette.close();

    // Verify no DOM nodes left behind
    const modal = document.querySelector('.command-palette-modal');
    expect(modal).toBeFalsy();

    const backdrop = document.querySelector('.command-palette-backdrop');
    expect(backdrop).toBeFalsy();
  });

  // Test 6: Multiple palette instances (independent state)
  it('supports multiple independent palette instances', () => {
    const container1 = document.createElement('div');
    const container2 = document.createElement('div');
    container1.id = 'palette-1';
    container2.id = 'palette-2';
    document.body.appendChild(container1);
    document.body.appendChild(container2);

    const palette1 = new CommandPalette('#palette-1', MOCK_SCHEMA);
    const palette2 = new CommandPalette('#palette-2', MOCK_SCHEMA);

    palette1.open();
    palette2.open();

    const modals = document.querySelectorAll('.command-palette-modal');
    expect(modals.length).toBe(2);

    palette1.close();
    const remainingModals = document.querySelectorAll('.command-palette-modal');
    expect(remainingModals.length).toBe(1);

    palette2.close();
    const finalModals = document.querySelectorAll('.command-palette-modal');
    expect(finalModals.length).toBe(0);

    document.body.removeChild(container1);
    document.body.removeChild(container2);
  });

  // Test 7: Close palette when backdrop is clicked
  it('closes palette when backdrop is clicked', () => {
    palette = new CommandPalette('#palette-container', MOCK_SCHEMA);
    palette.open();

    const backdrop = document.querySelector('.command-palette-backdrop') as HTMLElement;
    backdrop.click();

    const modal = document.querySelector('.command-palette-modal');
    expect(modal).toBeFalsy();
  });

  // Test 8: Form submission closes palette
  it('closes palette and resets after form submission', async () => {
    palette = new CommandPalette('#palette-container', MOCK_SCHEMA);
    palette.open();

    // Select first command
    const firstItem = document.querySelector('.command-item') as HTMLElement;
    firstItem.click();

    // Get form and submit
    const form = document.querySelector('form') as HTMLFormElement;
    const submitEvent = new Event('submit', { bubbles: true, cancelable: true });

    form.dispatchEvent(submitEvent);

    // After submission, palette should show command list again
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Verify command list is shown (form submission resets UI)
    const commandList = document.querySelector('.command-palette-list');
    expect(commandList?.style.display).not.toBe('none');

    // Verify form is cleared from DOM
    const formAfterSubmit = document.querySelector('form');
    expect(formAfterSubmit).toBeFalsy();

    // Verify form container is hidden
    const formContainer = document.querySelector('.command-palette-form-container');
    expect(formContainer?.style.display).toBe('none');

    // Verify search input is cleared
    const searchInput = document.querySelector('.command-palette-search') as HTMLInputElement;
    expect(searchInput?.value).toBe('');
  });
});
