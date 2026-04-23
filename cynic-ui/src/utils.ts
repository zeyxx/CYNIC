import { DEFAULT_API_BASE } from './types';

export const KERNEL_URL_LS_KEY = 'cynic_kernel_url';
export const SELECTED_DOGS_LS_KEY = 'cynic_selected_dogs';

export function getKernelUrl(): string {
  return localStorage.getItem(KERNEL_URL_LS_KEY) ?? DEFAULT_API_BASE;
}

const DEFAULT_DOGS = ['deterministic-dog', 'gemini-cli'];

export function getSelectedDogs(): string[] {
  const stored = localStorage.getItem(SELECTED_DOGS_LS_KEY);
  if (!stored) return DEFAULT_DOGS;
  const parsed: string[] = JSON.parse(stored);
  return parsed.length > 0 ? parsed : DEFAULT_DOGS;
}
