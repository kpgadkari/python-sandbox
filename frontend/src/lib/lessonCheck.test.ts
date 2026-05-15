import { describe, expect, it } from 'vitest';
import { expectedOutputMatches, normalizeStdout } from './lessonCheck';

describe('lesson output checks', () => {
  it('normalizes line endings and trailing newlines', () => {
    expect(normalizeStdout('hello\r\n')).toBe('hello');
    expect(expectedOutputMatches('hello\n', 'hello')).toBe(true);
  });

  it('does not ignore meaningful content differences', () => {
    expect(expectedOutputMatches('hello', 'hello!')).toBe(false);
  });
});
