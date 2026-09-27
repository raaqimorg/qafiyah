import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/server/client', () => ({ apiServer: { GET: vi.fn() } }));

import { apiServer } from '@/lib/server/client';

import { loadSearchFilterOptions } from './search-filter-options';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const row = (slug: string, name: string, poemsCount: number) => ({ slug, name, poemsCount });

const LISTS: Readonly<Record<string, readonly ReturnType<typeof row>[]>> = {
  '/eras': [row('jahili', 'جاهلي', 5)],
  '/meters': [row('alrajz', 'الرجز', 10), row('altawil', 'الطويل', 5)],
  '/rhymes': [row('meem', 'ميم', 3)],
  '/themes': [row('alnasib', 'النسيب', 2)],
  '/collections': [row('almuallaqat', 'المعلقات', 10)],
};

function answer(failing?: string) {
  get.mockImplementation((path: string) =>
    path === failing
      ? { error: { status: 500 }, response: new Response(null, { status: 500 }) }
      : { data: { data: LISTS[path] }, response: new Response(null, { status: 200 }) }
  );
}

describe('loadSearchFilterOptions', () => {
  beforeEach(() => {
    get.mockReset();
    answer();
  });

  it('turns each taxonomy list into select options with their poem counts', async () => {
    const options = (await loadSearchFilterOptions())._unsafeUnwrap();
    expect(options.eras).toEqual([{ value: 'jahili', label: 'جاهلي', poemsCount: 5 }]);
    expect(options.collections).toEqual([
      { value: 'almuallaqat', label: 'المعلقات', poemsCount: 10 },
    ]);
  });

  it('orders the meters classically, the way the filter shows them', async () => {
    const options = (await loadSearchFilterOptions())._unsafeUnwrap();
    expect(options.meters.map((option) => option.value)).toEqual(['altawil', 'alrajz']);
  });

  it('fails as a whole when one list cannot be fetched, so the page can skip caching', async () => {
    answer('/themes');
    expect((await loadSearchFilterOptions()).isErr()).toBe(true);
  });
});
