import { describe, expect, it } from 'vitest';
import { projectFileTarget } from './pythonWorker';

describe('projectFileTarget', () => {
  it('keeps top-level project files inside the Pyodide project root', () => {
    expect(projectFileTarget('main.py')).toEqual({
      normalized: 'main.py',
      directory: null,
    });
  });

  it('normalizes Windows separators and returns nested directories', () => {
    expect(projectFileTarget('lessons\\intro\\main.py')).toEqual({
      normalized: 'lessons/intro/main.py',
      directory: 'lessons/intro',
    });
  });

  it('rejects traversal and empty paths', () => {
    expect(() => projectFileTarget('../secret.py')).toThrow('Invalid project path');
    expect(() => projectFileTarget('nested/../secret.py')).toThrow('Invalid project path');
    expect(() => projectFileTarget('')).toThrow('Invalid project path');
  });
});
