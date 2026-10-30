import { describe, expect, it } from 'vitest';

import { canonicalPoetSearch, loadPoetSearchParams } from './poet-search-params';

describe('loadPoetSearchParams', () => {
  it('reads each repeated filter into its own list, and the cursor', () => {
    expect(loadPoetSearchParams('?meter=altawil&meter=alkamil&theme=alnasib&cursor=c')).toEqual({
      meter: ['altawil', 'alkamil'],
      rhyme: [],
      theme: ['alnasib'],
      cursor: 'c',
    });
  });

  it('starts on the first page with no filters when the query is empty', () => {
    expect(loadPoetSearchParams('')).toEqual({ meter: [], rhyme: [], theme: [], cursor: null });
  });
});

describe('canonicalPoetSearch', () => {
  it('writes the filters and cursor in one fixed order, whatever order the address bar had', () => {
    const search = new URLSearchParams('cursor=c&theme=alnasib&meter=altawil');
    expect(canonicalPoetSearch(search).toString()).toBe('meter=altawil&theme=alnasib&cursor=c');
  });

  it('drops an old page number', () => {
    expect(canonicalPoetSearch(new URLSearchParams('page=3')).toString()).toBe('');
  });
});
