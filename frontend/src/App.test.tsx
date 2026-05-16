import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { api } from './lib/api';
import type { User, WorkerEvent } from './lib/types';

vi.mock('@uiw/react-codemirror', () => ({
  default: ({ value, onChange }: { value: string; onChange: (value: string) => void }) => (
    <textarea aria-label="code editor" value={value} onChange={(event) => onChange(event.currentTarget.value)} />
  ),
}));

vi.mock('./lib/api', () => ({
  api: {
    me: vi.fn(),
    login: vi.fn(),
    logout: vi.fn(),
    listProjects: vi.fn(),
    createProject: vi.fn(),
    getProject: vi.fn(),
    saveProjectFiles: vi.fn(),
    deleteProject: vi.fn(),
    listLessons: vi.fn(),
    getLesson: vi.fn(),
    checkLesson: vi.fn(),
  },
}));

const parentUser = {
  id: 'parent-1',
  username: 'parent',
  display_name: 'Parent',
  role: 'parent' as const,
};

const childUser = {
  id: 'child-1',
  username: 'son',
  display_name: 'Young Coder',
  role: 'child' as const,
};

const lessonSummary = {
  id: 'hello-python',
  title: 'Hello, Python',
  prompt: 'Print a greeting.',
  description: 'Use print() to write text.',
  difficulty: 'Basics',
};

const lessonDetail = {
  ...lessonSummary,
  hint: 'Use quotation marks.',
  starter_code: 'print("hello, python")\n',
  expected_stdout: 'hello, python\n',
};

const project = {
  id: 'project-1',
  title: 'First Project',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  files: { 'main.py': 'print("from project")\n' },
};

class MockWorker {
  static latest: MockWorker | null = null;

  onmessage: ((message: MessageEvent<WorkerEvent>) => void) | null = null;
  onerror: ((error: ErrorEvent) => void) | null = null;
  posted: unknown[] = [];
  terminated = false;

  constructor() {
    MockWorker.latest = this;
  }

  postMessage(message: unknown) {
    this.posted.push(message);
  }

  terminate() {
    this.terminated = true;
  }

  emit(event: WorkerEvent) {
    this.onmessage?.({ data: event } as MessageEvent<WorkerEvent>);
  }
}

function mockWorkspace(user: User = childUser) {
  vi.mocked(api.me).mockResolvedValue({ user });
  vi.mocked(api.login).mockResolvedValue({ user });
  vi.mocked(api.logout).mockResolvedValue(undefined);
  vi.mocked(api.listLessons).mockResolvedValue([lessonSummary]);
  vi.mocked(api.getLesson).mockResolvedValue(lessonDetail);
  vi.mocked(api.checkLesson).mockResolvedValue({ passed: true, expected_stdout: 'hello, python\n' });
  vi.mocked(api.listProjects).mockResolvedValue([project]);
  vi.mocked(api.getProject).mockResolvedValue(project);
  vi.mocked(api.createProject).mockResolvedValue(project);
  vi.mocked(api.saveProjectFiles).mockResolvedValue(project);
}

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    MockWorker.latest = null;
    vi.stubGlobal('Worker', MockWorker);
    vi.stubGlobal('crypto', { randomUUID: () => 'run-1' });
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it('shows child-friendly login defaults when no session exists', async () => {
    vi.mocked(api.me).mockRejectedValue(new Error('no session'));

    render(<App />);

    expect(await screen.findByRole('heading', { name: 'Python Sandbox' })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Young Coder/ })).toBeTruthy();
    expect(screen.getByLabelText('Username')).toHaveProperty('value', 'son');
    expect(screen.getByLabelText('Password')).toHaveProperty('value', 'python');

    fireEvent.click(screen.getByRole('button', { name: /Parent/ }));
    expect(screen.getByLabelText('Username')).toHaveProperty('value', 'parent');
    expect(screen.getByLabelText('Password')).toHaveProperty('value', 'change-me');
  });

  it('logs child users into a lesson-only workspace and checks a run', async () => {
    mockWorkspace(childUser);
    vi.mocked(api.me).mockRejectedValue(new Error('no session'));
    vi.mocked(api.login).mockResolvedValue({ user: childUser });

    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: 'Sign in' }));

    expect(await screen.findByRole('heading', { name: 'Hello, Python', level: 2 })).toBeTruthy();
    expect(screen.queryByText('Projects')).toBeNull();
    expect(screen.getByText('Use print() to write text.')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    const worker = MockWorker.latest;
    expect(worker?.posted[0]).toMatchObject({
      type: 'run',
      runId: 'run-1',
      entrypoint: 'main.py',
      files: { 'main.py': 'print("hello, python")\n' },
    });

    act(() => {
      worker?.emit({ type: 'started', runId: 'run-1' });
      worker?.emit({ type: 'stdout', runId: 'run-1', text: 'hello, python\n' });
      worker?.emit({ type: 'result', runId: 'run-1', status: 'ok', durationMs: 12 });
    });

    await waitFor(() => expect(api.checkLesson).toHaveBeenCalledWith('hello-python', 'print("hello, python")\n', 'hello, python\n'));
    expect(await screen.findByText('Passed')).toBeTruthy();
  });

  it('loads parent projects and saves edited code', async () => {
    mockWorkspace(parentUser);

    render(<App />);

    expect(await screen.findByText('Projects')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Save' })).toBeTruthy();
    fireEvent.change(screen.getByLabelText('code editor'), {
      target: { value: 'print("updated")\n' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(api.saveProjectFiles).toHaveBeenCalledWith('project-1', {
        'main.py': 'print("updated")\n',
      }),
    );
  });

  it('shows login failures and signs out from an active session', async () => {
    mockWorkspace(parentUser);
    vi.mocked(api.me).mockRejectedValue(new Error('no session'));
    vi.mocked(api.login).mockRejectedValue(new Error('bad password'));

    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: 'Sign in' }));

    expect(await screen.findByText('bad password')).toBeTruthy();

    vi.mocked(api.login).mockResolvedValue({ user: parentUser });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'change-me' } });
    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }));

    expect(await screen.findByText('Projects')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Sign out' }));

    await waitFor(() => expect(api.logout).toHaveBeenCalled());
    expect(await screen.findByRole('heading', { name: 'Python Sandbox' })).toBeTruthy();
  });

  it('lets parents create projects, switch lessons, and reset starter code', async () => {
    const secondLesson = {
      id: 'variables',
      title: 'Variables',
      prompt: 'Store a name.',
      description: 'Create a variable and print it.',
      difficulty: 'Basics',
    };
    const secondLessonDetail = {
      ...secondLesson,
      hint: '',
      starter_code: 'name = "Ada"\nprint(name)\n',
      expected_stdout: 'Ada\n',
    };
    const createdProject = {
      ...project,
      id: 'project-2',
      title: 'Project 2',
      files: { 'main.py': 'print("new")\n' },
    };
    mockWorkspace(parentUser);
    vi.mocked(api.listLessons).mockResolvedValue([lessonSummary, secondLesson]);
    vi.mocked(api.getLesson).mockImplementation(async (id) => (id === 'variables' ? secondLessonDetail : lessonDetail));
    vi.mocked(api.createProject).mockResolvedValue(createdProject);
    vi.mocked(api.listProjects).mockResolvedValueOnce([project]).mockResolvedValue([project, createdProject]);

    render(<App />);

    expect(await screen.findByText('Projects')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'New project' }));

    await waitFor(() => expect(api.createProject).toHaveBeenCalledWith('Project 2'));
    await waitFor(() => expect(screen.getByLabelText('code editor')).toHaveProperty('value', 'print("new")\n'));

    fireEvent.click(screen.getByRole('button', { name: /Variables/ }));
    await waitFor(() => expect(screen.getByLabelText('code editor')).toHaveProperty('value', 'name = "Ada"\nprint(name)\n'));

    fireEvent.change(screen.getByLabelText('code editor'), { target: { value: 'print("changed")\n' } });
    fireEvent.click(screen.getByRole('button', { name: 'Reset' }));
    expect(screen.getByLabelText('code editor')).toHaveProperty('value', 'name = "Ada"\nprint(name)\n');
  });

  it('surfaces worker stderr, input prompts, ready state, and manual stops', async () => {
    mockWorkspace(childUser);

    render(<App />);

    expect(await screen.findByRole('heading', { name: 'Hello, Python', level: 2 })).toBeTruthy();
    fireEvent.change(screen.getByLabelText('Input lines'), { target: { value: 'Ada\nLovelace' } });
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));

    const worker = MockWorker.latest;
    expect(worker?.posted[0]).toMatchObject({ stdin: ['Ada', 'Lovelace'] });

    act(() => {
      worker?.emit({ type: 'ready' });
      worker?.emit({ type: 'started', runId: 'run-1' });
      worker?.emit({ type: 'stderr', runId: 'run-1', text: 'boom\n' });
      worker?.emit({ type: 'input_request', runId: 'run-1', prompt: '' });
    });

    expect(screen.getByText('Ready')).toBeTruthy();
    expect(screen.getByText('boom')).toBeTruthy();
    expect(screen.getByText('Input required. Add input lines and run again.')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Stop' }));
    expect(worker?.terminated).toBe(true);
    expect(screen.getByText('Run stopped.')).toBeTruthy();
  });
});
