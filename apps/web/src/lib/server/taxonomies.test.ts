import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { EraSlug, MeterSlug } from '@/lib/api/brands';

vi.mock('./client', () => ({
  apiServer: { GET: vi.fn() },
}));

import { failure, ok } from '@/test/api-results';

import { apiServer } from './client';
import { allEras, getMeter, getRhyme, getTheme } from './taxonomies';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const ERA = { name: 'الجاهلي', slug: 'jahili' as EraSlug, poemsCount: 31, poetsCount: 12 };
const METER = { name: 'الطويل', slug: 'altawil' as MeterSlug, poemsCount: 12, poetsCount: 9 };

beforeEach(() => {
  get.mockReset();
});

describe('allEras', () => {
  it('returns the list data', async () => {
    get.mockResolvedValue(ok({ data: [ERA] }));
    const eras = await allEras();
    expect(eras).toHaveLength(1);
    expect(eras[0]?.poetsCount).toBe(12);
    expect(get).toHaveBeenCalledWith('/eras');
  });
});

describe('getMeter', () => {
  it('returns the unwrapped meter on success', async () => {
    get.mockResolvedValue(ok({ data: METER }));
    expect(await getMeter('altawil' as MeterSlug)).toEqual(METER);
    expect(get).toHaveBeenCalledWith('/meters/{slug}', {
      params: { path: { slug: 'altawil' } },
    });
  });

  it('returns null on a 404', async () => {
    get.mockResolvedValue(failure(404));
    expect(await getMeter('missing' as MeterSlug)).toBeNull();
  });

  it('rethrows on a 500', async () => {
    get.mockResolvedValue(failure(500));
    await expect(getMeter('boom' as MeterSlug)).rejects.toThrow();
  });
});

describe('getRhyme and getTheme', () => {
  it('delegate to the matching endpoint', async () => {
    get.mockResolvedValue(ok({ data: METER }));
    await getRhyme('meem' as never);
    expect(get).toHaveBeenCalledWith('/rhymes/{slug}', {
      params: { path: { slug: 'meem' } },
    });
    await getTheme('alnasib' as never);
    expect(get).toHaveBeenCalledWith('/themes/{slug}', {
      params: { path: { slug: 'alnasib' } },
    });
  });
});
