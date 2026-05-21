import { afterEach, describe, expect, it, vi } from 'vitest';

function nextTick() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

type MockPyodide = {
  FS: {
    mkdirTree: ReturnType<typeof vi.fn>;
    writeFile: ReturnType<typeof vi.fn>;
  };
  runPython: ReturnType<typeof vi.fn>;
  runPythonAsync: ReturnType<typeof vi.fn>;
  setStdin: ReturnType<typeof vi.fn>;
  setStdout: ReturnType<typeof vi.fn>;
  setStderr: ReturnType<typeof vi.fn>;
};

async function loadWorkerWithMocks(pyodide: MockPyodide) {
  vi.resetModules();

  class MockWorkerScope {}
  const postMessage = vi.fn();
  const scope = new (MockWorkerScope as unknown as { new (): WorkerGlobalScope & { postMessage: typeof postMessage } })();
  Object.assign(scope, { postMessage, onmessage: null });

  vi.stubGlobal('WorkerGlobalScope', MockWorkerScope);
  vi.stubGlobal('self', scope);
  vi.stubGlobal('performance', { now: () => 100 });

  vi.doMock('/pyodide/pyodide.mjs?v=0.29.4', () => ({
    loadPyodide: vi.fn().mockResolvedValue(pyodide),
  }));

  await import('./pythonWorker');
  await nextTick();

  return { scope, postMessage };
}

describe('pythonWorker runtime', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
    vi.clearAllMocks();
  });

  it('boots Pyodide and executes a run request with project files', async () => {
    const pyodide: MockPyodide = {
      FS: {
        mkdirTree: vi.fn(),
        writeFile: vi.fn(),
      },
      runPython: vi.fn(),
      runPythonAsync: vi.fn().mockResolvedValue(undefined),
      setStdin: vi.fn(),
      setStdout: vi.fn(),
      setStderr: vi.fn(),
    };

    const { scope, postMessage } = await loadWorkerWithMocks(pyodide);
    expect(postMessage).toHaveBeenCalledWith({ type: 'ready' });

    scope.onmessage?.({
      data: {
        type: 'run',
        runId: 'run-1',
        entrypoint: 'main.py',
        files: {
          'main.py': 'print("hello")\n',
          'lessons/intro/helper.py': 'value = 1\n',
        },
        stdin: ['Ada'],
      },
    } as MessageEvent);

    await nextTick();

    expect(pyodide.FS.mkdirTree).toHaveBeenCalledWith('/home/pyodide/project');
    expect(pyodide.FS.mkdirTree).toHaveBeenCalledWith('/home/pyodide/project/lessons/intro');
    expect(pyodide.FS.writeFile).toHaveBeenCalledWith('/home/pyodide/project/main.py', 'print("hello")\n');
    expect(pyodide.FS.writeFile).toHaveBeenCalledWith('/home/pyodide/project/lessons/intro/helper.py', 'value = 1\n');
    expect(pyodide.runPython).toHaveBeenCalledWith(expect.stringContaining('os.chdir("/home/pyodide/project")'));
    expect(pyodide.runPythonAsync).toHaveBeenCalledWith('print("hello")\n');
    expect(postMessage).toHaveBeenCalledWith({ type: 'started', runId: 'run-1' });
    expect(postMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'result',
        runId: 'run-1',
        status: 'ok',
      }),
    );
  });

  it('requests input when stdin is empty and reports Python execution errors', async () => {
    const pyodide: MockPyodide = {
      FS: {
        mkdirTree: vi.fn(),
        writeFile: vi.fn(),
      },
      runPython: vi.fn(),
      runPythonAsync: vi.fn().mockRejectedValue(new Error('python exploded')),
      setStdin: vi.fn(),
      setStdout: vi.fn(),
      setStderr: vi.fn(),
    };

    const { scope, postMessage } = await loadWorkerWithMocks(pyodide);

    scope.onmessage?.({
      data: {
        type: 'run',
        runId: 'run-2',
        entrypoint: 'main.py',
        files: { 'main.py': 'print(input())\n' },
        stdin: [],
      },
    } as MessageEvent);

    await nextTick();

    expect(pyodide.setStdin).toHaveBeenCalledTimes(1);
    const stdinProvider = pyodide.setStdin.mock.calls[0][0].stdin as () => string;
    expect(() => stdinProvider()).toThrow('Input required. Add input text and run again.');
    expect(postMessage).toHaveBeenCalledWith({ type: 'input_request', runId: 'run-2', prompt: '' });
    expect(postMessage).toHaveBeenCalledWith({
      type: 'stderr',
      runId: 'run-2',
      text: 'python exploded\n',
    });
    expect(postMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'result',
        runId: 'run-2',
        status: 'error',
      }),
    );
  });
});
