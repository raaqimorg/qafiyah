import { describe, expect, it } from 'vitest';

import { canonicalPoetSearch, loadPoetSearchParams } from './poet-search-params';

describe('loadPoetSearchParams', () => {
  it('reads each repeated filter into its own list, and the page', () => {
    expect(loadPoetSearchParams('?meter=altawil&meter=alkamil&theme=alnasib&page=2')).toEqual({
      meter: ['altawil', 'alkamil'],
      rhyme: [],
      theme: ['alnasib'],
      page: 2,
    });
  });

  it('starts on the first page with no filters when the query is empty', () => {
    expect(loadPoetSearchParams('')).toEqual({ meter: [], rhyme: [], theme: [], page: 1 });
  });
});

describe('canonicalPoetSearch', () => {
  it('writes the filters and page in one fixed order, whatever order the address bar had', () => {
    const search = new URLSearchParams('page=3&theme=alnasib&meter=altawil');
    expect(canonicalPoetSearch(search).toString()).toBe('meter=altawil&theme=alnasib&page=3');
  });

  it('leaves out the first page', () => {
    expect(canonicalPoetSearch(new URLSearchParams('page=1')).toString()).toBe('');
  });
});
