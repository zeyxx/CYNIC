export type { OrganismSchema, CommandSchema } from './api';

export interface CommandParam {
  type: 'string' | 'number' | 'boolean' | 'enum';
  required: boolean;
  description: string;
  enum?: string[];
  default?: string | number | boolean;
}
