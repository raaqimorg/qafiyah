import { describe, expect, it } from 'vitest';

import { excerptAtWordBoundary, sanitizeMetaText, truncateMetaText, withBrand } from './meta-text';

describe('sanitizeMetaText', () => {
  it('strips quotes and backslashes and collapses whitespace', () => {
    expect(sanitizeMetaText('شاعرة "سورية"  معاصرة\\')).toBe('شاعرة سورية معاصرة');
  });

  it('trims surrounding whitespace', () => {
    expect(sanitizeMetaText('  نص  ')).toBe('نص');
  });
});

describe('truncateMetaText', () => {
  it('keeps text at or under the optimal length unchanged', () => {
    expect(truncateMetaText('قصير')).toBe('قصير');
  });

  it('cuts anything longer to exactly the optimal length', () => {
    const long = 'x'.repeat(400);
    expect(truncateMetaText(long)).toHaveLength(300);
  });
});

describe('excerptAtWordBoundary', () => {
  it('returns the whole value when it fits', () => {
    expect(excerptAtWordBoundary('كلمة واحدة', 20)).toBe('كلمة واحدة');
  });

  it('cuts back to the last space so it never splits a word', () => {
    const cut = excerptAtWordBoundary('one two three four', 14);
    expect(cut).toBe('one two three');
    expect(cut.length).toBeLessThanOrEqual(14);
  });

  it('returns the raw cut when there is no space to cut on', () => {
    expect(excerptAtWordBoundary('abcdefghij', 5)).toBe('abcde');
  });
});

describe('withBrand', () => {
  it('appends the site brand', () => {
    expect(withBrand('ديوان المتنبي')).toBe('ديوان المتنبي | قافية');
  });
});
