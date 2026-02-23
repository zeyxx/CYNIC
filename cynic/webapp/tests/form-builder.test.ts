import { describe, it, expect, vi, beforeEach } from 'vitest';
import { SchemaCache, CACHE_KEY, createFormFromSchema, createFormField } from '../src/ui/form-builder';
import type { CommandParam } from '../src/types/schema';
import type { CommandSchema, OrganismSchema } from '../src/types/api';

const MOCK_SCHEMA: OrganismSchema = {
  version: '1.0.0',
  commands: [],
  skills: [],
  state: {},
};

function schemaResponse(schema: OrganismSchema): Response {
  return new Response(JSON.stringify(schema), { status: 200, headers: { 'Content-Type': 'application/json' } });
}

const MOCK_COMMAND: CommandSchema = {
  id: 'get-status',
  name: 'get-status',
  description: 'Fetch organism status',
  params: {
    verbose: { type: 'boolean', required: false, description: 'Include extra detail' } as CommandParam,
    format: { type: 'enum', required: true, description: 'Output format', enum: ['json', 'text'] } as CommandParam,
  },
  returns: { type: 'object', description: 'Status object' },
};

describe('SchemaCache', () => {
  beforeEach(() => {
    SchemaCache.invalidate();
    localStorage.clear();
  });

  it('load() fetches from /api/organism/schema', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(schemaResponse(MOCK_SCHEMA));
    const result = await SchemaCache.load('/api/organism/schema');
    expect(fetchSpy).toHaveBeenCalledOnce();
    expect(result.version).toBe('1.0.0');
  });

  it('caches result in localStorage under CACHE_KEY', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(schemaResponse(MOCK_SCHEMA));
    await SchemaCache.load('/api/organism/schema');
    const raw = localStorage.getItem(CACHE_KEY);
    expect(raw).not.toBeNull();
  });

  it('returns localStorage copy without network call when TTL is valid', async () => {
    const freshEntry = { schema: MOCK_SCHEMA, expiry: Date.now() + 1_800_000 };
    localStorage.setItem(CACHE_KEY, JSON.stringify(freshEntry));
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    const result = await SchemaCache.load('/api/organism/schema');
    expect(fetchSpy).not.toHaveBeenCalled();
    expect(result.version).toBe('1.0.0');
  });

  it('invalidate() removes localStorage entry', () => {
    localStorage.setItem(CACHE_KEY, 'test');
    SchemaCache.invalidate();
    expect(localStorage.getItem(CACHE_KEY)).toBeNull();
  });
});

describe('createFormFromSchema', () => {
  it('returns HTMLFormElement with id "form-{command.name}"', () => {
    const form = createFormFromSchema(MOCK_COMMAND);
    expect(form).toBeInstanceOf(HTMLFormElement);
    expect(form.id).toBe('form-get-status');
  });

  it('contains a submit button', () => {
    const form = createFormFromSchema(MOCK_COMMAND);
    const btn = form.querySelector('button[type="submit"]');
    expect(btn).not.toBeNull();
  });
});

describe('createFormField - string type', () => {
  it('creates <input type="text"> with correct id', () => {
    const param: CommandParam = { type: 'string', required: false, description: 'Query' };
    const fieldset = createFormField('query', param);
    const input = fieldset.querySelector<HTMLInputElement>('input[type="text"]');
    expect(input).not.toBeNull();
    expect(input?.id).toBe('query');
  });
});

describe('createFormField - enum type', () => {
  it('creates <select> with options', () => {
    const param: CommandParam = { type: 'enum', required: true, description: 'Format', enum: ['json', 'text'] };
    const fieldset = createFormField('format', param);
    const select = fieldset.querySelector<HTMLSelectElement>('select');
    expect(select).not.toBeNull();
    expect(select?.options.length).toBeGreaterThan(0);
  });
});

describe('createFormField - boolean type', () => {
  it('creates <input type="checkbox">', () => {
    const param: CommandParam = { type: 'boolean', required: false, description: 'Verbose' };
    const fieldset = createFormField('verbose', param);
    const checkbox = fieldset.querySelector<HTMLInputElement>('input[type="checkbox"]');
    expect(checkbox).not.toBeNull();
  });
});

describe('createFormField - number type', () => {
  it('creates <input type="number">', () => {
    const param: CommandParam = { type: 'number', required: false, description: 'Count' };
    const fieldset = createFormField('count', param);
    const input = fieldset.querySelector<HTMLInputElement>('input[type="number"]');
    expect(input).not.toBeNull();
  });
});
