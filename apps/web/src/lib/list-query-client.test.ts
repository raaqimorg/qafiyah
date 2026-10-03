import { describe, expect, it, vi } from 'vitest';

import { createListQueryClient } from './list-query-client';

describe('createListQueryClient', () => {
  it('loads the page the reader asked for in full when its list fails to fetch', async () => {
    const loadPage = vi.fn();
    const client = createListQueryClient(loadPage);
    await client
      .query({
        queryKey: ['poems'],
        queryFn: () => Promise.reject(new Error('proxy returned 429')),
        meta: { pageUrl: '/poets/oNbs?meter=altawil' },
      })
      .catch(() => undefined);
    expect(loadPage).toHaveBeenCalledExactlyOnceWith('/poets/oNbs?meter=altawil');
  });

  it('does not retry a failed fetch before falling back', async () => {
    const queryFn = vi.fn(() => Promise.reject(new Error('offline')));
    const client = createListQueryClient(vi.fn());
    await client
      .query({ queryKey: ['poets'], queryFn, meta: { pageUrl: '/poets' } })
      .catch(() => undefined);
    expect(queryFn).toHaveBeenCalledOnce();
  });

  it('leaves a successful fetch alone', async () => {
    const loadPage = vi.fn();
    const client = createListQueryClient(loadPage);
    await client.query({
      queryKey: ['poets'],
      queryFn: () => Promise.resolve([]),
      meta: { pageUrl: '/poets' },
    });
    expect(loadPage).not.toHaveBeenCalled();
  });
});
