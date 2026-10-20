import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./client', () => ({
  apiServer: { GET: vi.fn() },
}));

import { failure, ok } from '@/test/api-results';

import { apiServer } from './client';
import { allFeelings, feelingVerses } from './feelings';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const PAGINATION = { page: 1, pageSize: 30, totalPages: 1, totalItems: 1 };
const FEELING = { name: 'الحزن', slug: 'alhuzn', wheelTerm: 'Sad', verseCount: 1 };

beforeEach(() => {
  get.mockReset();
});

describe('allFeelings', () => {
  it('returns the list data', async () => {
    get.mockResolvedValue(ok({ data: [FEELING], pagination: PAGINATION }));
    expect(await allFeelings()).toEqual([FEELING]);
    expect(get).toHaveBeenCalledWith('/feelings');
  });
});

describe('feelingVerses', () => {
  it('asks for the page of the feeling', async () => {
    get.mockResolvedValue(ok({ data: [], pagination: PAGINATION }));
    expect(await feelingVerses('alhuzn', 1)).toEqual({ verses: [], pagination: PAGINATION });
    expect(get).toHaveBeenCalledWith('/feelings/{slug}/verses', {
      params: { path: { slug: 'alhuzn' }, query: { page: '1' } },
    });
  });

  it('returns null on a 404, and past the last page', async () => {
    get.mockResolvedValue(failure(404));
    expect(await feelingVerses('alnothing', 1)).toBeNull();
    get.mockResolvedValue(ok({ data: [], pagination: { ...PAGINATION, page: 9 } }));
    expect(await feelingVerses('alhuzn', 9)).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(feelingVerses('alhuzn', 1)).rejects.toThrow();
  });
});
