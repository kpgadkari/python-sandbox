import { afterEach, describe, expect, it, vi } from 'vitest';
import { newRunId } from './runId';

describe('newRunId', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('uses crypto.randomUUID when available', () => {
    vi.stubGlobal('crypto', {
      randomUUID: () => 'uuid-from-api',
      getRandomValues: vi.fn(),
    });

    expect(newRunId()).toBe('uuid-from-api');
  });

  it('falls back to getRandomValues when randomUUID is unavailable', () => {
    const bytes = Uint8Array.from({ length: 16 }, (_, index) => index);
    const getRandomValues = vi.fn((target: Uint8Array) => {
      target.set(bytes);
      return target;
    });
    vi.stubGlobal('crypto', { getRandomValues });

    expect(newRunId()).toBe('00010203-0405-4607-8809-0a0b0c0d0e0f');
    expect(getRandomValues).toHaveBeenCalled();
  });

  it('falls back to time and Math.random when crypto is missing', () => {
    vi.stubGlobal('crypto', undefined);
    vi.spyOn(Date, 'now').mockReturnValue(1_700_000_000_000);
    vi.spyOn(Math, 'random').mockReturnValue(0.123456789);

    expect(newRunId()).toBe('run-1700000000000-1f9add3739635f');
  });
});
