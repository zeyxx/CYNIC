export const AUTH_SESSION_LS_KEY = 'cynic_auth_session';

export interface AuthSession {
  session_token: string;
  role: string;
  expires_at: number;
  address: string;
}

export function buildAuthMessage(input: { domain: string; statement: string; nonce: string }): string {
  return [
    'CYNIC AUTH',
    `domain:${input.domain}`,
    `nonce:${input.nonce}`,
    `statement:${input.statement}`,
  ].join('\n');
}

export function loadAuthSession(): AuthSession | null {
  if (typeof window === 'undefined') return null;
  const raw = window.localStorage.getItem(AUTH_SESSION_LS_KEY);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as AuthSession;
    if (!parsed?.session_token || !parsed?.expires_at || !parsed?.address) return null;
    if (parsed.expires_at * 1000 <= Date.now()) {
      window.localStorage.removeItem(AUTH_SESSION_LS_KEY);
      return null;
    }
    return parsed;
  } catch {
    window.localStorage.removeItem(AUTH_SESSION_LS_KEY);
    return null;
  }
}

export function saveAuthSession(session: AuthSession): void {
  if (typeof window === 'undefined') return;
  window.localStorage.setItem(AUTH_SESSION_LS_KEY, JSON.stringify(session));
}

export function clearAuthSession(): void {
  if (typeof window === 'undefined') return;
  window.localStorage.removeItem(AUTH_SESSION_LS_KEY);
}

export function getAuthSessionToken(): string | null {
  return loadAuthSession()?.session_token ?? null;
}
