import { afterEach, describe, expect, it, vi } from 'vitest';
import { api } from './api';

function jsonResponse(body: unknown, init: ResponseInit = {}) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
    ...init,
  });
}

describe('api client', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('sends JSON requests with session credentials', async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ passed: true, expected_stdout: 'ok\n' }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(api.checkLesson('hello-python', 'print("ok")', 'ok\n')).resolves.toEqual({
      passed: true,
      expected_stdout: 'ok\n',
    });

    expect(fetchMock).toHaveBeenCalledWith('/api/lessons/hello-python/check', {
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
      body: JSON.stringify({ code_snapshot: 'print("ok")', stdout: 'ok\n' }),
    });
  });

  it('returns undefined for empty successful responses', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 204 })));

    await expect(api.logout()).resolves.toBeUndefined();
  });

  it('uses API error messages when requests fail', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ error: 'unauthorized' }, { status: 401 })));

    await expect(api.me()).rejects.toThrow('unauthorized');
  });

  it('falls back to HTTP status text when error responses are not JSON', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('nope', { status: 500, statusText: 'Server Error' })));

    await expect(api.listProjects()).rejects.toThrow('500 Server Error');
  });
});
