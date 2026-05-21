import type { RunRequest, WorkerEvent } from '../lib/types';
import { loadPyodideModule, type PyodideInterface } from './pyodideLoader';

let pyodideReady: Promise<PyodideInterface> | null = null;

export function projectFileTarget(path: string) {
  const normalized = path.replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  if (parts.length === 0 || parts.some((part) => part === '..')) {
    throw new Error(`Invalid project path: ${path}`);
  }
  return {
    normalized,
    directory: parts.length > 1 ? parts.slice(0, -1).join('/') : null,
  };
}

function post(event: WorkerEvent) {
  self.postMessage(event);
}

async function getPyodide() {
  if (!pyodideReady) {
    pyodideReady = loadPyodideModule().catch((error) => {
      pyodideReady = null;
      throw error;
    });
  }
  return pyodideReady;
}

async function runPython(request: RunRequest) {
  const started = performance.now();
  const stdin = [...(request.stdin ?? [])];

  try {
    const pyodide = await getPyodide();
    post({ type: 'started', runId: request.runId });

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

    pyodide.FS.mkdirTree('/home/pyodide/project');
    for (const [path, contents] of Object.entries(request.files)) {
      const target = projectFileTarget(path);
      if (target.directory) {
        pyodide.FS.mkdirTree(`/home/pyodide/project/${target.directory}`);
      }
      pyodide.FS.writeFile(`/home/pyodide/project/${target.normalized}`, contents);
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

export function initializeWorkerRuntime(): void {
  getPyodide()
    .then(() => post({ type: 'ready' }))
    .catch((error) => console.error(error));

  self.onmessage = (message: MessageEvent<RunRequest>) => {
    if (message.data.type === 'run') {
      void runPython(message.data);
    }
  };
}

if (
  typeof self !== 'undefined' &&
  typeof WorkerGlobalScope !== 'undefined' &&
  self instanceof WorkerGlobalScope
) {
  initializeWorkerRuntime();
}
