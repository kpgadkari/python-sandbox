import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from 'react';
import CodeMirror from '@uiw/react-codemirror';
import { python } from '@codemirror/lang-python';
import { oneDark } from '@codemirror/theme-one-dark';
import {
  BookOpen,
  CheckCircle2,
  FileCode2,
  Loader2,
  LogOut,
  Play,
  Plus,
  RotateCcw,
  Save,
  Square,
  Terminal,
  XCircle,
} from 'lucide-react';
import { api } from './lib/api';
import type { LessonDetail, LessonSummary, ProjectDetail, ProjectSummary, User, WorkerEvent } from './lib/types';

const MAX_OUTPUT_BYTES = 64 * 1024;
const RUN_TIMEOUT_MS = 5000;

type ConsoleLine = {
  kind: 'stdout' | 'stderr' | 'system';
  text: string;
};

type LessonResult = {
  passed: boolean;
  expected: string;
} | null;

export function App() {
  const [user, setUser] = useState<User | null>(null);
  const [loginError, setLoginError] = useState('');
  const [loading, setLoading] = useState(true);
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [project, setProject] = useState<ProjectDetail | null>(null);
  const [lessons, setLessons] = useState<LessonSummary[]>([]);
  const [lesson, setLesson] = useState<LessonDetail | null>(null);
  const [code, setCode] = useState('print("hello, python")\n');
  const [stdin, setStdin] = useState('');
  const [consoleLines, setConsoleLines] = useState<ConsoleLine[]>([]);
  const [runState, setRunState] = useState<'idle' | 'loading' | 'running'>('idle');
  const [workerReady, setWorkerReady] = useState(false);
  const [saveState, setSaveState] = useState<'idle' | 'saving' | 'saved'>('idle');
  const [lessonResult, setLessonResult] = useState<LessonResult>(null);
  const workerRef = useRef<Worker | null>(null);
  const timeoutRef = useRef<number | null>(null);
  const stdoutRef = useRef('');
  const outputBytesRef = useRef(0);
  const codeRef = useRef(code);
  const lessonRef = useRef<LessonDetail | null>(lesson);

  const files = useMemo(() => ({ ...(project?.files ?? {}), 'main.py': code }), [project?.files, code]);

  useEffect(() => {
    codeRef.current = code;
  }, [code]);

  useEffect(() => {
    lessonRef.current = lesson;
  }, [lesson]);

  const appendConsole = useCallback((kind: ConsoleLine['kind'], text: string) => {
    setConsoleLines((current) => [...current, { kind, text }]);
  }, []);

  const stopWorker = useCallback((status: 'timeout' | 'stopped') => {
    if (timeoutRef.current) {
      window.clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    workerRef.current?.terminate();
    workerRef.current = null;
    setRunState('idle');
    setWorkerReady(false);
    appendConsole('system', status === 'timeout' ? 'Run stopped after 5 seconds.\n' : 'Run stopped.\n');
  }, [appendConsole]);

  const ensureWorker = useCallback(() => {
    if (workerRef.current) {
      return workerRef.current;
    }

    setWorkerReady(false);
    const worker = new Worker(new URL('./workers/pythonWorker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = async (message: MessageEvent<WorkerEvent>) => {
      const event = message.data;
      if (event.type === 'ready') {
        setWorkerReady(true);
        return;
      }
      if (event.type === 'started') {
        if (timeoutRef.current) {
          window.clearTimeout(timeoutRef.current);
        }
        timeoutRef.current = window.setTimeout(() => stopWorker('timeout'), RUN_TIMEOUT_MS);
        setRunState('running');
        return;
      }
      if (event.type === 'stdout' || event.type === 'stderr') {
        outputBytesRef.current += event.text.length;
        if (outputBytesRef.current > MAX_OUTPUT_BYTES) {
          stopWorker('stopped');
          appendConsole('system', 'Output limit reached.\n');
          return;
        }
        if (event.type === 'stdout') {
          stdoutRef.current += event.text;
        }
        appendConsole(event.type, event.text);
        return;
      }
      if (event.type === 'input_request') {
        appendConsole('system', 'Input required. Add input lines and run again.\n');
        return;
      }
      if (event.type === 'result') {
        if (timeoutRef.current) {
          window.clearTimeout(timeoutRef.current);
          timeoutRef.current = null;
        }
        setRunState('idle');
        appendConsole(
          'system',
          event.status === 'ok'
            ? `Finished in ${event.durationMs} ms.\n`
            : `Stopped with ${event.status} after ${event.durationMs} ms.\n`,
        );
        if (lessonRef.current && event.status === 'ok') {
          try {
            const result = await api.checkLesson(lessonRef.current.id, codeRef.current, stdoutRef.current);
            setLessonResult({ passed: result.passed, expected: result.expected_stdout });
          } catch (error) {
            appendConsole('system', `Could not check lesson: ${error instanceof Error ? error.message : String(error)}\n`);
          }
        }
      }
    };
    worker.onerror = (error) => {
      if (timeoutRef.current) {
        window.clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      appendConsole('stderr', `${error.message}\n`);
      setRunState('idle');
    };
    workerRef.current = worker;
    return worker;
  }, [appendConsole, stopWorker]);

  const loadWorkspace = useCallback(async () => {
    const [projectList, lessonList] = await Promise.all([api.listProjects(), api.listLessons()]);
    setProjects(projectList);
    setLessons(lessonList);
    if (projectList.length > 0) {
      const detail = await api.getProject(projectList[0].id);
      setProject(detail);
      setCode(detail.files['main.py'] ?? '');
    } else {
      const created = await api.createProject('First Project');
      setProject(created);
      setCode(created.files['main.py'] ?? '');
      setProjects(await api.listProjects());
    }
    if (lessonList.length > 0) {
      setLesson(await api.getLesson(lessonList[0].id));
    }
  }, []);

  useEffect(() => {
    api
      .me()
      .then(async ({ user }) => {
        setUser(user);
        await loadWorkspace();
      })
      .catch(() => {
        setUser(null);
      })
      .finally(() => setLoading(false));

    return () => workerRef.current?.terminate();
  }, [loadWorkspace]);

  async function handleLogin(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoginError('');
    const form = new FormData(event.currentTarget);
    try {
      const { user } = await api.login(String(form.get('username')), String(form.get('password')));
      setUser(user);
      await loadWorkspace();
    } catch (error) {
      setLoginError(error instanceof Error ? error.message : 'Login failed');
    }
  }

  async function handleLogout() {
    await api.logout();
    setUser(null);
    setProject(null);
    setProjects([]);
    setLessons([]);
    setLesson(null);
  }

  async function createProject() {
    const created = await api.createProject(`Project ${projects.length + 1}`);
    setProject(created);
    setCode(created.files['main.py'] ?? '');
    setProjects(await api.listProjects());
    setLessonResult(null);
  }

  async function selectProject(id: string) {
    const selected = await api.getProject(id);
    setProject(selected);
    setCode(selected.files['main.py'] ?? '');
    setLessonResult(null);
  }

  async function saveProject() {
    if (!project) {
      return;
    }
    setSaveState('saving');
    const saved = await api.saveProjectFiles(project.id, files);
    setProject(saved);
    setProjects(await api.listProjects());
    setSaveState('saved');
    window.setTimeout(() => setSaveState('idle'), 1200);
  }

  async function selectLesson(id: string) {
    const selected = await api.getLesson(id);
    setLesson(selected);
    setCode(selected.starter_code);
    setLessonResult(null);
    appendConsole('system', `Loaded lesson: ${selected.title}\n`);
  }

  function runCode() {
    if (runState !== 'idle') {
      return;
    }
    const worker = ensureWorker();
    const runId = crypto.randomUUID();
    stdoutRef.current = '';
    outputBytesRef.current = 0;
    setLessonResult(null);
    setConsoleLines([]);
    setRunState(workerReady ? 'running' : 'loading');

    worker.postMessage({
      type: 'run',
      runId,
      files,
      entrypoint: 'main.py',
      stdin: stdin.length > 0 ? stdin.split('\n') : [],
    });
  }

  function resetCode() {
    setCode(lesson?.starter_code ?? project?.files['main.py'] ?? 'print("hello, python")\n');
    setLessonResult(null);
    setConsoleLines([]);
  }

  if (loading) {
    return (
      <main className="center-screen">
        <Loader2 className="spin" size={28} />
      </main>
    );
  }

  if (!user) {
    return (
      <main className="login-screen">
        <form className="login-panel" onSubmit={handleLogin}>
          <div>
            <h1>Python Sandbox</h1>
            <p>Home coding space</p>
          </div>
          <label>
            Username
            <input name="username" defaultValue="parent" autoComplete="username" />
          </label>
          <label>
            Password
            <input name="password" type="password" autoComplete="current-password" />
          </label>
          {loginError ? <p className="error-text">{loginError}</p> : null}
          <button type="submit">Sign in</button>
        </form>
      </main>
    );
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <FileCode2 size={24} />
          <div>
            <h1>Python Sandbox</h1>
            <p>{user.display_name}</p>
          </div>
        </div>

        <section className="sidebar-section">
          <div className="section-title">
            <span>Projects</span>
            <button className="icon-button" type="button" onClick={createProject} aria-label="New project">
              <Plus size={16} />
            </button>
          </div>
          <div className="list">
            {projects.map((item) => (
              <button
                className={item.id === project?.id ? 'list-item active' : 'list-item'}
                type="button"
                key={item.id}
                onClick={() => void selectProject(item.id)}
              >
                {item.title}
              </button>
            ))}
          </div>
        </section>

        <section className="sidebar-section">
          <div className="section-title">
            <span>Lessons</span>
            <BookOpen size={16} />
          </div>
          <div className="list">
            {lessons.map((item) => (
              <button
                className={item.id === lesson?.id ? 'list-item active' : 'list-item'}
                type="button"
                key={item.id}
                onClick={() => void selectLesson(item.id)}
              >
                {item.title}
              </button>
            ))}
          </div>
        </section>

        <button className="logout-button" type="button" onClick={() => void handleLogout()}>
          <LogOut size={16} />
          Sign out
        </button>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <h2>{project?.title ?? 'Untitled Project'}</h2>
            <p>{lesson ? lesson.prompt : 'Write Python and run it safely in your browser.'}</p>
          </div>
          <div className="toolbar">
            <button type="button" onClick={runCode} disabled={runState !== 'idle'}>
              {runState === 'idle' ? <Play size={16} /> : <Loader2 className="spin" size={16} />}
              Run
            </button>
            <button type="button" onClick={() => stopWorker('stopped')} disabled={runState === 'idle'}>
              <Square size={16} />
              Stop
            </button>
            <button type="button" onClick={resetCode}>
              <RotateCcw size={16} />
              Reset
            </button>
            <button type="button" onClick={() => void saveProject()} disabled={!project || saveState === 'saving'}>
              <Save size={16} />
              {saveState === 'saved' ? 'Saved' : 'Save'}
            </button>
          </div>
        </header>

        <div className="main-grid">
          <section className="editor-pane">
            <div className="pane-title">main.py</div>
            <CodeMirror
              value={code}
              height="100%"
              theme={oneDark}
              extensions={[python()]}
              onChange={(value) => setCode(value)}
              basicSetup={{
                foldGutter: false,
                highlightActiveLine: true,
                autocompletion: true,
              }}
            />
          </section>

          <aside className="right-pane">
            <section className="console-panel">
              <div className="pane-title">
                <span>
                  <Terminal size={16} />
                  Console
                </span>
                <span className={workerReady ? 'status ready' : 'status'}>{workerReady ? 'Ready' : 'Boots on first run'}</span>
              </div>
              <div className="console-output">
                {consoleLines.length === 0 ? (
                  <p className="muted">Run your code to see output here.</p>
                ) : (
                  consoleLines.map((line, index) => (
                    <pre className={line.kind} key={`${line.kind}-${index}`}>
                      {line.text}
                    </pre>
                  ))
                )}
              </div>
            </section>

            <section className="input-panel">
              <label>
                Input lines
                <textarea
                  value={stdin}
                  onChange={(event) => setStdin(event.target.value)}
                  placeholder="Ada"
                  rows={4}
                />
              </label>
            </section>

            <section className="lesson-panel">
              <div className="pane-title">Lesson Check</div>
              {lessonResult ? (
                <div className={lessonResult.passed ? 'lesson-result passed' : 'lesson-result failed'}>
                  {lessonResult.passed ? <CheckCircle2 size={18} /> : <XCircle size={18} />}
                  <span>{lessonResult.passed ? 'Passed' : `Expected: ${JSON.stringify(lessonResult.expected)}`}</span>
                </div>
              ) : (
                <p className="muted">Select a lesson and run the code to check it.</p>
              )}
            </section>
          </aside>
        </div>
      </section>
    </main>
  );
}
