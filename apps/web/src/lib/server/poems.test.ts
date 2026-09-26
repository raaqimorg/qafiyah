import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { PoemSlug, PoetSlug } from '@/lib/api/brands';

vi.mock('./client', () => ({
  apiServer: { GET: vi.fn() },
}));

import { failure, ok } from '@/test/api-results';

import { apiServer } from './client';
import { getPoem, listPoems, movedPoemPath } from './poems';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const POEM = {
  title: 'قصيدة',
  slug: 'a-poem' as PoemSlug,
  verses: [['شطر', 'شطر']],
  verseCount: 1,
  sample: 'شطر',
  keywords: 'k',
  poet: { name: 'شاعر', slug: 'poet-x' },
  era: { name: 'الجاهلي', slug: 'jahili' },
  meter: { name: 'الطويل', slug: 'altaweel' },
  theme: { name: 'مدح', slug: 'theme-1' },
  relatedPoems: [],
};

const POEM_ROW = {
  title: 'ت',
  slug: 'p1',
  poet: { name: 'ش', slug: 'poet-x' },
  meter: { name: 'م', slug: 'meter-x' },
};
const PAGINATION = { page: 1, pageSize: 30, totalPages: 1, totalItems: 1 };

describe('getPoem', () => {
  beforeEach(() => {
    get.mockReset();
  });

  it('returns the unwrapped poem on success', async () => {
    get.mockResolvedValue(ok({ data: POEM }));
    const result = await getPoem('a-poem' as PoemSlug);
    expect(result).toEqual(POEM);
    expect(get).toHaveBeenCalledWith('/poems/{slug}', {
      params: { path: { slug: 'a-poem' }, query: {} },
    });
  });

  it('asks for neighbors within a grouping when one is named', async () => {
    get.mockResolvedValue(ok({ data: POEM }));
    await getPoem('a-poem' as PoemSlug, 'theme');
    expect(get).toHaveBeenCalledWith('/poems/{slug}', {
      params: { path: { slug: 'a-poem' }, query: { by: 'theme' } },
    });
  });

  it('returns null on a 404', async () => {
    get.mockResolvedValue(failure(404));
    expect(await getPoem('missing' as PoemSlug)).toBeNull();
  });

  it('returns null on a 400, which can never resolve either', async () => {
    get.mockResolvedValue(failure(400));
    expect(await getPoem('bad' as PoemSlug)).toBeNull();
  });

  it('rethrows on a 500 (a genuine server error is not a missing poem)', async () => {
    get.mockResolvedValue(failure(500));
    await expect(getPoem('boom' as PoemSlug)).rejects.toThrow();
  });

  it('rethrows when the request never got an answer', async () => {
    get.mockRejectedValue(new Error('network down'));
    await expect(getPoem('boom' as PoemSlug)).rejects.toThrow('network down');
  });
});

describe('listPoems', () => {
  beforeEach(() => {
    get.mockReset();
  });

  it('maps poems and pagination on success', async () => {
    get.mockResolvedValue(ok({ data: [POEM_ROW], pagination: PAGINATION }));
    const result = await listPoems({ poetSlugs: ['poet-x' as PoetSlug] }, 1);
    expect(result?.poems).toHaveLength(1);
    expect(result?.pagination.totalPages).toBe(1);
    expect(get).toHaveBeenCalledWith('/poems', {
      params: {
        query: {
          page: '1',
          poet: ['poet-x'],
          era: [],
          theme: [],
          meter: [],
          rhyme: [],
          collection: [],
        },
      },
    });
  });

  it('returns null when page is past the last page', async () => {
    get.mockResolvedValue(
      ok({ data: [], pagination: { page: 99, pageSize: 30, totalPages: 1, totalItems: 1 } })
    );
    expect(await listPoems({}, 99)).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(listPoems({}, 1)).rejects.toThrow();
  });
});

describe('movedPoemPath', () => {
  it('is null when the poem answered under the slug that was asked for', () => {
    expect(movedPoemPath('TnKK', { slug: 'TnKK' })).toBeNull();
  });

  it('is the canonical path when the API followed an alias to another slug', () => {
    expect(movedPoemPath('abCD', { slug: 'TnKK' })).toBe('/poems/TnKK');
  });
});
