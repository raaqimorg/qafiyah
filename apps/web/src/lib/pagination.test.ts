import { describe, expect, it } from 'vitest';

import { derivePagination, reachedFirstPage, readCursor } from './pagination';

describe('readCursor', () => {
  it('reads the cursor param', () => {
    expect(readCursor(new URL('https://x.test/meters/altawil?cursor=eyJhZnRlciI6MzB9'))).toBe(
      'eyJhZnRlciI6MzB9'
    );
  });
  it('treats an absent or empty cursor as the first page', () => {
    expect(readCursor(new URL('https://x.test/meters/altawil'))).toBeUndefined();
    expect(readCursor(new URL('https://x.test/meters/altawil?cursor='))).toBeUndefined();
  });
});

describe('derivePagination', () => {
  const makeUrl = (cursor: string) => `/x?cursor=${cursor}`;

  it('links both directions on a middle page', () => {
    expect(derivePagination({ next: 'n', prev: 'p' }, makeUrl)).toStrictEqual({
      nextUrl: '/x?cursor=n',
      prevUrl: '/x?cursor=p',
    });
  });

  it('links forward only on the first page', () => {
    expect(derivePagination({ next: 'n', prev: null }, makeUrl)).toStrictEqual({
      nextUrl: '/x?cursor=n',
      prevUrl: undefined,
    });
  });

  it('links back only on the last page', () => {
    expect(derivePagination({ next: null, prev: 'p' }, makeUrl)).toStrictEqual({
      nextUrl: undefined,
      prevUrl: '/x?cursor=p',
    });
  });
});

describe('reachedFirstPage', () => {
  it('is true when a cursor leads back to the first page', () => {
    expect(reachedFirstPage('p', { next: 'n', prev: null })).toBe(true);
  });
  it('is false for the bare first page and for any later page', () => {
    expect(reachedFirstPage(undefined, { next: 'n', prev: null })).toBe(false);
    expect(reachedFirstPage('n', { next: null, prev: 'p' })).toBe(false);
  });
});
