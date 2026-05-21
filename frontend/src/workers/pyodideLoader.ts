export type PyodideInterface = {
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

type PyodideModule = {
  loadPyodide: (options?: { indexURL?: string }) => Promise<PyodideInterface>;
};

const PYODIDE_INDEX_URL = '/pyodide/';
const PYODIDE_MODULE_URL = `${PYODIDE_INDEX_URL}pyodide.mjs?v=0.29.4`;

export async function loadPyodideModule(): Promise<PyodideInterface> {
  const module = await import(/* @vite-ignore */ PYODIDE_MODULE_URL);
  return (module as PyodideModule).loadPyodide({ indexURL: PYODIDE_INDEX_URL });
}
