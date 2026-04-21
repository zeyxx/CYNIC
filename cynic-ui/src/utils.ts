import { DEFAULT_API_BASE } from './types';

export const KERNEL_URL_LS_KEY = 'cynic_kernel_url';
export const SELECTED_DOGS_LS_KEY = 'cynic_selected_dogs';

export function getKernelUrl(): string {
  return localStorage.getItem(KERNEL_URL_LS_KEY) ?? DEFAULT_API_BASE;
}

export function getSelectedDogs(): string[] | undefined {
  const stored = localStorage.getItem(SELECTED_DOGS_LS_KEY);
  if (!stored) return undefined;
  const parsed: string[] = JSON.parse(stored);
  // Empty array means "all dogs" — return undefined so kernel omits the filter
  return parsed.length > 0 ? parsed : undefined;
}
