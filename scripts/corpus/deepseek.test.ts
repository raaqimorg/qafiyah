import { describe, expect, it } from 'bun:test';

import { isPeak, offPeakDollars } from './deepseek';

describe('isPeak', () => {
  it('is true on a weekday at 02:00 and at 07:00 UTC', () => {
    expect(isPeak(new Date('2026-10-14T02:00:00Z'))).toBe(true);
    expect(isPeak(new Date('2026-10-14T07:30:00Z'))).toBe(true);
  });

  it('is false on a weekday between the two peaks and in the evening', () => {
    expect(isPeak(new Date('2026-10-14T05:00:00Z'))).toBe(false);
    expect(isPeak(new Date('2026-10-14T20:00:00Z'))).toBe(false);
  });

  it('is false at the weekend', () => {
    expect(isPeak(new Date('2026-10-17T02:00:00Z'))).toBe(false);
  });
});

describe('offPeakDollars', () => {
  it('prices cached input, fresh input, and output tokens', () => {
    expect(
      offPeakDollars({
        prompt_cache_hit_tokens: 1_000_000,
        prompt_cache_miss_tokens: 1_000_000,
        completion_tokens: 1_000_000,
      })
    ).toBeCloseTo(0.753);
  });
});
