import type { LessonDetail, LessonSummary, ProjectDetail, ProjectSummary, User } from './types';

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(path, {
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...init.headers,
    },
    ...init,
  });

  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try {
      const body = await response.json();
      message = body.error ?? message;
    } catch {
      // Keep the HTTP fallback message.
    }
    throw new Error(message);
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return response.json() as Promise<T>;
}

export const api = {
  login(username: string, password: string) {
    return request<{ user: User }>('/api/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    });
  },
  me() {
    return request<{ user: User }>('/api/me');
  },
  logout() {
    return request<void>('/api/logout', { method: 'POST' });
  },
  listProjects() {
    return request<ProjectSummary[]>('/api/projects');
  },
  createProject(title?: string, starterCode?: string) {
    return request<ProjectDetail>('/api/projects', {
      method: 'POST',
      body: JSON.stringify({ title, starter_code: starterCode }),
    });
  },
  getProject(id: string) {
    return request<ProjectDetail>(`/api/projects/${id}`);
  },
  saveProjectFiles(id: string, files: Record<string, string>) {
    return request<ProjectDetail>(`/api/projects/${id}/files`, {
      method: 'PUT',
      body: JSON.stringify({ files }),
    });
  },
  deleteProject(id: string) {
    return request<void>(`/api/projects/${id}`, { method: 'DELETE' });
  },
  listLessons() {
    return request<LessonSummary[]>('/api/lessons');
  },
  getLesson(id: string) {
    return request<LessonDetail>(`/api/lessons/${id}`);
  },
  checkLesson(id: string, codeSnapshot: string, stdout: string) {
    return request<{ passed: boolean; expected_stdout: string }>(`/api/lessons/${id}/check`, {
      method: 'POST',
      body: JSON.stringify({ code_snapshot: codeSnapshot, stdout }),
    });
  },
};
