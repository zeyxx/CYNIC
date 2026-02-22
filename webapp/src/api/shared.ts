export class NetworkError extends Error {
  constructor(message: string, public status?: number) {
    super(message);
    this.name = 'NetworkError';
  }
}

export class ValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ValidationError';
  }
}

export class ServerError extends Error {
  constructor(message: string, public status?: number) {
    super(message);
    this.name = 'ServerError';
  }
}

export async function fetchWithTimeout<T>(
  url: string,
  options: RequestInit = {},
  timeoutMs: number = 5000
): Promise<T> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal,
    });

    if (!response.ok) {
      const statusMessage = response.status === 404 ? 'Not found' : response.statusText;
      throw new NetworkError(`HTTP ${response.status}: ${statusMessage}`, response.status);
    }

    let data;
    try {
      data = await response.json();
    } catch (e) {
      throw new ValidationError(`Invalid JSON response: ${(e as Error).message}`);
    }

    return data;
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new NetworkError(`Request timeout (${timeoutMs}ms)`);
    }
    throw e;
  } finally {
    clearTimeout(timeoutId);
  }
}
