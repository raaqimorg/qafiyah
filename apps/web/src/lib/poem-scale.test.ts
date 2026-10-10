import { describe, expect, it } from 'vitest';

import { FONT_SCALE, SPACING_SCALE, stepScale } from './poem-scale';

describe('stepScale', () => {
  it('moves one step up or down from the default', () => {
    expect(stepScale(1, 1, FONT_SCALE)).toBe(1.1);
    expect(stepScale(1, -1, SPACING_SCALE)).toBe(0.8);
  });

  it('rounds away floating-point noise after several steps', () => {
    expect(stepScale(stepScale(1, 1, FONT_SCALE), 1, FONT_SCALE)).toBe(1.2);
  });

  it('stops at the limits of the range', () => {
    expect(stepScale(FONT_SCALE.max, 1, FONT_SCALE)).toBe(FONT_SCALE.max);
    expect(stepScale(SPACING_SCALE.min, -1, SPACING_SCALE)).toBe(SPACING_SCALE.min);
  });
});
