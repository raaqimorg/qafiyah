import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./client', () => ({
  apiServer: { GET: vi.fn() },
}));

import { failure, ok } from '@/test/api-results';

import { apiServer } from './client';
import { allFeelings, feelingVerses } from './feelings';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const FEELING = { name: 'الحزن', slug: 'alhuzn', wheelTerm: 'Sad', verseCount: 1 };

beforeEach(() => {
  get.mockReset();
});

describe('allFeelings', () => {
  it('returns the list data', async () => {
    get.mockResolvedValue(ok({ data: [FEELING] }));
    expect(await allFeelings()).toEqual([FEELING]);
    expect(get).toHaveBeenCalledWith('/feelings');
  });
});

describe('feelingVerses', () => {
  it('asks for the page of the feeling', async () => {
    const page = { data: [], total: 0, next: null, prev: null };
    get.mockResolvedValue(ok(page));
    expect(await feelingVerses('alhuzn', undefined)).toEqual(page);
    expect(get).toHaveBeenCalledWith('/feelings/{slug}/verses', {
      params: { path: { slug: 'alhuzn' }, query: {} },
    });
    await feelingVerses('alhuzn', 'c');
    expect(get).toHaveBeenLastCalledWith('/feelings/{slug}/verses', {
      params: { path: { slug: 'alhuzn' }, query: { cursor: 'c' } },
    });
  });

  it('returns null on a 404, on a refused cursor, and past the last verse', async () => {
    get.mockResolvedValue(failure(404));
    expect(await feelingVerses('alnothing', undefined)).toBeNull();
    get.mockResolvedValue(failure(400));
    expect(await feelingVerses('alhuzn', 'bad')).toBeNull();
    get.mockResolvedValue(ok({ data: [], total: 40, next: null, prev: null }));
    expect(await feelingVerses('alhuzn', 'c')).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(feelingVerses('alhuzn', undefined)).rejects.toThrow();
  });
});
