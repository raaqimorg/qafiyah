import { describe, expect, it } from 'vitest';

import {
  canonicalPoetFilters,
  everyPoetFilterListed,
  hasPoetFilters,
  NO_POET_FILTERS,
  samePoetFilters,
} from './poet-filters';

const FACETS = {
  meters: [{ slug: 'altawil' }, { slug: 'alkamil' }],
  rhymes: [{ slug: 'meem' }],
  themes: [{ slug: 'alnasib' }],
};

describe('canonicalPoetFilters', () => {
  it('sorts each list and drops repeats', () => {
    expect(
      canonicalPoetFilters({ meter: ['altawil', 'alkamil', 'altawil'], rhyme: ['noon'], theme: [] })
    ).toEqual({ meter: ['alkamil', 'altawil'], rhyme: ['noon'], theme: [] });
  });
});

describe('hasPoetFilters', () => {
  it('is true once any list has a value', () => {
    expect(hasPoetFilters(NO_POET_FILTERS)).toBe(false);
    expect(hasPoetFilters({ ...NO_POET_FILTERS, rhyme: ['meem'] })).toBe(true);
  });
});

describe('everyPoetFilterListed', () => {
  it('accepts a selection whose every value the facets list', () => {
    const selection = { meter: ['altawil', 'alkamil'], rhyme: [], theme: ['alnasib'] };
    expect(everyPoetFilterListed(selection, FACETS)).toBe(true);
  });

  it('rejects a selection with a value the facets do not list', () => {
    const selection = { meter: ['altawil'], rhyme: ['lam'], theme: [] };
    expect(everyPoetFilterListed(selection, FACETS)).toBe(false);
  });
});

describe('samePoetFilters', () => {
  it('ignores the order of values within a list', () => {
    const a = { meter: ['altawil', 'alkamil'], rhyme: [], theme: [] };
    const b = { meter: ['alkamil', 'altawil'], rhyme: [], theme: [] };
    expect(samePoetFilters(a, b)).toBe(true);
  });

  it('tells a changed list apart', () => {
    const a = { meter: ['altawil'], rhyme: [], theme: [] };
    expect(samePoetFilters(a, { ...a, meter: [] })).toBe(false);
    expect(samePoetFilters(a, { ...a, theme: ['alnasib'] })).toBe(false);
  });
});
