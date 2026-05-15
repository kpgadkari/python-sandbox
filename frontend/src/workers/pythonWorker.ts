import type { RunRequest, WorkerEvent } from '../lib/types';

type PyodideInterface = {
  FS: {
    mkdirTree(path: string): void;
    writeFile(path: string, contents: string): void;
  };
  runPython(code: string): unknown;
  runPythonAsync(code: string): Promise<unknown>;
  setStdin(options: { stdin: () => string }): void;
  setStdout(options: { batched: (text: string) => void }): void;
  setStderr(options: { batched: (text: string) => void }): void;
};

declare const loadPyodide: (options?: { indexURL?: string }) => Promise<PyodideInterface>;

const PYODIDE_INDEX_URL = '/pyodide/';

let pyodideReady: Promise<PyodideInterface> | null = null;

function post(event: WorkerEvent) {
  self.postMessage(event);
}

async function getPyodide() {
  if (!pyodideReady) {
    self.importScripts(`${PYODIDE_INDEX_URL}pyodide.js`);
    pyodideReady = loadPyodide({ indexURL: PYODIDE_INDEX_URL });
  }
  return pyodideReady;
}

async function runPython(request: RunRequest) {
  const started = performance.now();
  const pyodide = await getPyodide();
  const stdin = [...(request.stdin ?? [])];

  pyodide.setStdout({
    batched: (text: string) => post({ type: 'stdout', runId: request.runId, text: `${text}\n` }),
  });
  pyodide.setStderr({
    batched: (text: string) => post({ type: 'stderr', runId: request.runId, text: `${text}\n` }),
  });
  pyodide.setStdin({
    stdin: () => {
      if (stdin.length === 0) {
        post({ type: 'input_request', runId: request.runId, prompt: '' });
        throw new Error('Input required. Add input text and run again.');
      }
      return `${stdin.shift() ?? ''}\n`;
    },
  });

  try {
    pyodide.FS.mkdirTree('/home/pyodide/project');
    for (const [path, contents] of Object.entries(request.files)) {
      const normalized = path.replace(/\\/g, '/');
      const parts = normalized.split('/').filter(Boolean);
      if (parts.some((part) => part === '..')) {
        throw new Error(`Invalid project path: ${path}`);
      }
      if (parts.length > 1) {
        pyodide.FS.mkdirTree(`/home/pyodide/project/${parts.slice(0, -1).join('/')}`);
      }
      pyodide.FS.writeFile(`/home/pyodide/project/${normalized}`, contents);
    }

    pyodide.runPython(`
import os
os.chdir("/home/pyodide/project")
`);
    const code = request.files[request.entrypoint] ?? '';
    await pyodide.runPythonAsync(code);
    post({
      type: 'result',
      runId: request.runId,
      status: 'ok',
      durationMs: Math.round(performance.now() - started),
    });
  } catch (error) {
    post({
      type: 'stderr',
      runId: request.runId,
      text: error instanceof Error ? `${error.message}\n` : `${String(error)}\n`,
    });
    post({
      type: 'result',
      runId: request.runId,
      status: 'error',
      durationMs: Math.round(performance.now() - started),
    });
  }
}

getPyodide().then(() => post({ type: 'ready' }));

self.onmessage = (message: MessageEvent<RunRequest>) => {
  if (message.data.type === 'run') {
    void runPython(message.data);
  }
};
