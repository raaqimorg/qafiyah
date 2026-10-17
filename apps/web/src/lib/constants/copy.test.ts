import { describe, expect, it } from 'vitest';

import { formatPoemCopySignature } from './copy';

describe('formatPoemCopySignature', () => {
  it('puts the poet, site and canonical URL on one attribution line', () => {
    expect(formatPoemCopySignature('امرؤ القيس', 'https://qafiyah.com/poems/rHUD')).toBe(
      'امرؤ القيس | قافية: https://qafiyah.com/poems/rHUD'
    );
  });
});
