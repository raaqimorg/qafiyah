import { describe, expect, it } from 'vitest';

import { halvesSpacing, POEM_SCALE, stepScale } from './poem-scale';

describe('stepScale', () => {
  it('moves one step up or down from the default', () => {
    expect(stepScale(1, 1, POEM_SCALE.font)).toBe(1.1);
    expect(stepScale(1, -1, POEM_SCALE.spacing)).toBe(0.8);
  });

  it('rounds away floating-point noise after several steps', () => {
    expect(stepScale(stepScale(1, 1, POEM_SCALE.font), 1, POEM_SCALE.font)).toBe(1.2);
  });

  it('stops at the limits of the range', () => {
    expect(stepScale(POEM_SCALE.font.max, 1, POEM_SCALE.font)).toBe(POEM_SCALE.font.max);
    expect(stepScale(POEM_SCALE.spacing.min, -1, POEM_SCALE.spacing)).toBe(POEM_SCALE.spacing.min);
  });
});

describe('halvesSpacing', () => {
  it('keeps the halves of a verse at their floor when only the verse gap can shrink', () => {
    expect(halvesSpacing(POEM_SCALE.spacing.min)).toBe(POEM_SCALE.spacing.halvesMin);
    expect(halvesSpacing(POEM_SCALE.spacing.halvesMin)).toBe(POEM_SCALE.spacing.halvesMin);
  });

  it('follows the spacing above the floor', () => {
    expect(halvesSpacing(1.4)).toBe(1.4);
  });
});
