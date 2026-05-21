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
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ passed: true }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(api.checkLesson('hello-python', 'print("ok")', 'ok\n')).resolves.toEqual({
      passed: true,
    });

    expect(fetchMock).toHaveBeenCalledWith('/api/lessons/hello-python/check', {
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
      body: JSON.stringify({ code_snapshot: 'print("ok")', stdout: 'ok\n' }),
    });
  });

  it('maps each API helper to the expected endpoint', async () => {
    const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(jsonResponse({ id: 'project-1' })));
    vi.stubGlobal('fetch', fetchMock);

    await api.login('son', 'python');
    await api.me();
    await api.listProjects();
    await api.createProject('Loops', 'print("go")');
    await api.getProject('project-1');
    await api.saveProjectFiles('project-1', { 'main.py': 'print("saved")\n' });
    await api.deleteProject('project-1');
    await api.listLessons();
    await api.getLesson('lesson-1');

    expect(fetchMock).toHaveBeenNthCalledWith(1, '/api/login', expect.objectContaining({
      method: 'POST',
      body: JSON.stringify({ username: 'son', password: 'python' }),
    }));
    expect(fetchMock).toHaveBeenNthCalledWith(2, '/api/me', expect.objectContaining({ credentials: 'include' }));
    expect(fetchMock).toHaveBeenNthCalledWith(3, '/api/projects', expect.objectContaining({ credentials: 'include' }));
    expect(fetchMock).toHaveBeenNthCalledWith(4, '/api/projects', expect.objectContaining({
      method: 'POST',
      body: JSON.stringify({ title: 'Loops', starter_code: 'print("go")' }),
    }));
    expect(fetchMock).toHaveBeenNthCalledWith(5, '/api/projects/project-1', expect.objectContaining({ credentials: 'include' }));
    expect(fetchMock).toHaveBeenNthCalledWith(6, '/api/projects/project-1/files', expect.objectContaining({
      method: 'PUT',
      body: JSON.stringify({ files: { 'main.py': 'print("saved")\n' } }),
    }));
    expect(fetchMock).toHaveBeenNthCalledWith(7, '/api/projects/project-1', expect.objectContaining({ method: 'DELETE' }));
    expect(fetchMock).toHaveBeenNthCalledWith(8, '/api/lessons', expect.objectContaining({ credentials: 'include' }));
    expect(fetchMock).toHaveBeenNthCalledWith(9, '/api/lessons/lesson-1', expect.objectContaining({ credentials: 'include' }));
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
