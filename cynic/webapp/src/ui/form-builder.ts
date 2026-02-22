import type { OrganismSchema, CommandSchema } from '../types/api';
import type { CommandParam } from '../types/schema';

export const CACHE_KEY = 'cynic-schema-cache';
const CACHE_TTL = 3_600_000;

interface CacheEntry {
  schema: OrganismSchema;
  expiry: number;
}

export class SchemaCache {
  private static instance: OrganismSchema | null = null;
  private static instanceExpiry: number = 0;

  static async load(url: string = '/api/organism/schema'): Promise<OrganismSchema> {
    const now = Date.now();

    if (SchemaCache.instance !== null && now < SchemaCache.instanceExpiry) {
      return SchemaCache.instance;
    }

    try {
      const raw = localStorage.getItem(CACHE_KEY);
      if (raw !== null) {
        const entry = JSON.parse(raw) as CacheEntry;
        if (now < entry.expiry) {
          SchemaCache.instance = entry.schema;
          SchemaCache.instanceExpiry = entry.expiry;
          return entry.schema;
        }
        localStorage.removeItem(CACHE_KEY);
      }
    } catch {
      // Fall through to fetch
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch schema: HTTP ${response.status}`);
    }

    const schema = (await response.json()) as OrganismSchema;
    const expiry = now + CACHE_TTL;

    SchemaCache.instance = schema;
    SchemaCache.instanceExpiry = expiry;

    try {
      localStorage.setItem(CACHE_KEY, JSON.stringify({ schema, expiry }));
    } catch {
      // Storage unavailable
    }

    return schema;
  }

  static invalidate(): void {
    SchemaCache.instance = null;
    SchemaCache.instanceExpiry = 0;
    try {
      localStorage.removeItem(CACHE_KEY);
    } catch {
      // Storage unavailable
    }
  }
}

export function createFormFromSchema(
  command: CommandSchema,
  containerId?: string
): HTMLFormElement {
  const form = document.createElement('form');
  form.id = `form-${command.name}`;
  form.setAttribute('data-command', command.name);

  const title = document.createElement('h3');
  title.textContent = command.name;
  form.appendChild(title);

  if (command.description) {
    const desc = document.createElement('p');
    desc.className = 'form-description';
    desc.textContent = command.description;
    form.appendChild(desc);
  }

  const params = command.params as Record<string, CommandParam>;
  for (const [paramName, param] of Object.entries(params)) {
    form.appendChild(createFormField(paramName, param));
  }

  const submitBtn = document.createElement('button');
  submitBtn.type = 'submit';
  submitBtn.textContent = `Invoke ${command.name}`;
  form.appendChild(submitBtn);

  if (containerId) {
    document.getElementById(containerId)?.appendChild(form);
  }

  return form;
}

export function createFormField(name: string, param: CommandParam): HTMLFieldSetElement {
  const fieldset = document.createElement('fieldset');
  fieldset.className = `field field-${param.type}`;

  const label = document.createElement('label');
  label.htmlFor = name;
  label.textContent = name;
  if (param.required) {
    const reqMark = document.createElement('span');
    reqMark.className = 'required';
    reqMark.textContent = ' *';
    label.appendChild(reqMark);
  }
  fieldset.appendChild(label);

  let control: HTMLInputElement | HTMLSelectElement;

  if (param.type === 'enum') {
    const select = document.createElement('select');
    select.id = name;
    select.name = name;

    const blank = document.createElement('option');
    blank.value = '';
    blank.textContent = '-- select --';
    select.appendChild(blank);

    if (param.enum) {
      for (const optValue of param.enum) {
        const opt = document.createElement('option');
        opt.value = optValue;
        opt.textContent = optValue;
        if (param.default !== undefined && String(param.default) === optValue) {
          opt.selected = true;
        }
        select.appendChild(opt);
      }
    }

    if (param.required) {
      select.required = true;
    }
    control = select;

  } else if (param.type === 'boolean') {
    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.id = name;
    checkbox.name = name;
    if (param.default === true) {
      checkbox.checked = true;
    }
    control = checkbox;

  } else if (param.type === 'number') {
    const numInput = document.createElement('input');
    numInput.type = 'number';
    numInput.id = name;
    numInput.name = name;
    if (param.default !== undefined) {
      numInput.value = String(param.default);
    }
    if (param.required) {
      numInput.required = true;
    }
    control = numInput;

  } else {
    const textInput = document.createElement('input');
    textInput.type = 'text';
    textInput.id = name;
    textInput.name = name;
    if (param.default !== undefined) {
      textInput.value = String(param.default);
    }
    if (param.required) {
      textInput.required = true;
    }
    control = textInput;
  }

  fieldset.appendChild(control);

  if (param.description) {
    const help = document.createElement('small');
    help.className = 'help-text';
    help.textContent = param.description;
    fieldset.appendChild(help);
  }

  return fieldset;
}
