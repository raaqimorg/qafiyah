import { describe, expect, it } from 'vitest';

import { searchQueryKey, searchQueryParams } from './search-query';

const base = {
  q: 'حب',
  eras: [],
  meters: [],
  rhymes: [],
  themes: [],
  poemTypes: [],
  collections: [],
  exact: false,
} as const;

describe('searchQueryParams', () => {
  it('sends only the requested section and its cursor', () => {
    expect(
      searchQueryParams({ ...base, type: 'poems', cursor: 'c', eras: ['jahili'] })
    ).toStrictEqual({
      q: 'حب',
      types: ['poems'],
      poemsCursor: 'c',
      eraSlugs: ['jahili'],
      meterSlugs: [],
      rhymeSlugs: [],
      themeSlugs: [],
      poemTypeSlugs: [],
      collectionSlugs: [],
      poetSlugs: [],
      exact: 'false',
    });
  });

  it('sends the verse form filter for poems', () => {
    expect(
      searchQueryParams({ ...base, type: 'poems', poemTypes: ['hurr'] }).poemTypeSlugs
    ).toStrictEqual(['hurr']);
  });

  it('pages the poets section on its own param', () => {
    const params = searchQueryParams({ ...base, type: 'poets', cursor: 'c' });
    expect(params.poetsCursor).toBe('c');
    expect(params).not.toHaveProperty('poemsCursor');
    expect(params.types).toStrictEqual(['poets']);
  });

  it('sends no cursor for the first page', () => {
    const params = searchQueryParams({ ...base, type: 'poems' });
    expect(params).not.toHaveProperty('poemsCursor');
    expect(params).not.toHaveProperty('poetsCursor');
  });

  it('blanks the poem-only facets for a poets request, which the API rejects', () => {
    const params = searchQueryParams({
      ...base,
      type: 'poets',
      meters: ['altawil'],
      rhymes: ['meem'],
      themes: ['alnasib'],
      poemTypes: ['hurr'],
      collections: ['almuallaqat'],
      eras: ['jahili'],
    });
    expect(params.meterSlugs).toStrictEqual([]);
    expect(params.rhymeSlugs).toStrictEqual([]);
    expect(params.themeSlugs).toStrictEqual([]);
    expect(params.poemTypeSlugs).toStrictEqual([]);
    expect(params.collectionSlugs).toStrictEqual([]);
    expect(params.eraSlugs).toStrictEqual(['jahili']);
  });

  it('passes exact through as the literal the API declares', () => {
    expect(searchQueryParams({ ...base, type: 'poems', exact: true }).exact).toBe('true');
  });
});

describe('searchQueryKey', () => {
  it('keys poems and poets separately', () => {
    expect(searchQueryKey({ ...base, type: 'poems' })).not.toStrictEqual(
      searchQueryKey({ ...base, type: 'poets' })
    );
  });

  it('does not vary with the cursor, which useInfiniteQuery manages', () => {
    expect(searchQueryKey({ ...base, type: 'poems' })).toStrictEqual(
      searchQueryKey({ ...base, type: 'poems', cursor: 'c' })
    );
  });
});
