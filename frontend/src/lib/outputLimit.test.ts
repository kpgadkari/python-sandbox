import { describe, expect, it } from 'vitest';
import { OutputLimiter } from './outputLimit';

describe('OutputLimiter', () => {
  it('accepts output up to the configured limit', () => {
    const limiter = new OutputLimiter(5);
    expect(limiter.accept('he')).toBe(true);
    expect(limiter.accept('llo')).toBe(true);
    expect(limiter.accept('!')).toBe(false);
  });

  it('can be reset between runs', () => {
    const limiter = new OutputLimiter(3);
    expect(limiter.accept('abcd')).toBe(false);
    limiter.reset();
    expect(limiter.accept('abc')).toBe(true);
  });
});
