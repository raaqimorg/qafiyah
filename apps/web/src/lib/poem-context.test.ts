import { describe, expect, it } from 'vitest';

import { parsePoemContext, resolvePoemContext } from './poem-context';

import type { Poem } from '@/lib/api/result-types';

const poem = {
  meter: { name: 'الطويل', slug: 'altawil' },
  rhyme: { name: 'الراء', slug: 'r' },
  theme: { name: 'الحماسة', slug: 'alhamasa' },
} as unknown as Poem;

describe('parsePoemContext', () => {
  it('reads each taxonomy section', () => {
    expect(parsePoemContext('meters')).toBe('meters');
    expect(parsePoemContext('rhymes')).toBe('rhymes');
    expect(parsePoemContext('themes')).toBe('themes');
    expect(parsePoemContext('collections')).toBe('collections');
  });

  it('ignores a missing or unknown value', () => {
    expect(parsePoemContext(null)).toBeUndefined();
    expect(parsePoemContext('')).toBeUndefined();
    expect(parsePoemContext('poets')).toBeUndefined();
    expect(parsePoemContext('Themes')).toBeUndefined();
  });
});

describe('resolvePoemContext', () => {
  it('keeps a listing the poem belongs to', () => {
    expect(resolvePoemContext(poem, 'themes')).toBe('themes');
    expect(resolvePoemContext(poem, undefined)).toBeUndefined();
  });

  it('drops a collection context when the poem is in no collection', () => {
    expect(resolvePoemContext(poem, 'collections')).toBeUndefined();
    const collected = { ...poem, collection: { name: 'المعلقات', slug: 'almuallaqat' } } as Poem;
    expect(resolvePoemContext(collected, 'collections')).toBe('collections');
  });
});
