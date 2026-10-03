import { experimental_AstroContainer as AstroContainer } from 'astro/container';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { SSRManifest } from 'astro';

vi.mock('@/lib/server/client', () => ({ apiServer: { GET: vi.fn() } }));
vi.mock('@/lib/server/cache', () => ({ CACHE_HTML: 'cache-html', CACHE_NONE: 'cache-none' }));
vi.mock('@/lib/observability/report-error', () => ({ reportError: vi.fn() }));

import { apiServer } from '@/lib/server/client';
import { failure } from '@/test/api-results';

import CollectionPage from './collections/[slug].astro';
import MeterPage from './meters/[slug].astro';
import RhymePage from './rhymes/[slug].astro';
import ThemePage from './themes/[slug].astro';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const PAGES = [
  ['meters', MeterPage],
  ['rhymes', RhymePage],
  ['themes', ThemePage],
  ['collections', CollectionPage],
] as const;

async function render(page: (typeof PAGES)[number][1], url: string) {
  const container = await AstroContainer.create({
    manifest: { trailingSlash: 'never' } as SSRManifest,
  });
  return await container.renderToResponse(page, {
    request: new Request(url),
    params: { slug: 'altawil' },
  });
}

beforeEach(() => {
  get.mockReset();
  get.mockImplementation(() => failure(404));
});

describe.each(PAGES)('GET /%s/[slug]', (section, page) => {
  const base = `https://qafiyah.com/${section}/altawil`;

  it.each([
    ['?utm_source=x', `/${section}/altawil`],
    ['?page=02', `/${section}/altawil?page=2`],
    ['?page=2&page=3', `/${section}/altawil?page=2`],
  ])('redirects %s to the canonical URL without calling the API', async (query, canonical) => {
    const response = await render(page, `${base}${query}`);
    expect(response.status).toBe(301);
    expect(response.headers.get('location')).toBe(canonical);
    expect(get).not.toHaveBeenCalled();
  });

  it('sends the canonical URL itself on to the API', async () => {
    await render(page, `${base}?page=2`).catch(() => undefined);
    expect(get).toHaveBeenCalled();
  });
});
