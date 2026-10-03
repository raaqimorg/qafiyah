import { getContainerRenderer } from '@astrojs/react/container-renderer';
import { experimental_AstroContainer as AstroContainer } from 'astro/container';
import { loadRenderers } from 'astro:container';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { SSRManifest } from 'astro';

vi.mock('@/lib/server/client', () => ({ apiServer: { GET: vi.fn() } }));
vi.mock('@/lib/server/cache', () => ({ CACHE_HTML: 'cache-html', CACHE_NONE: 'cache-none' }));
vi.mock('@/lib/observability/report-error', () => ({ reportError: vi.fn() }));

import { reportError } from '@/lib/observability/report-error';
import { apiServer } from '@/lib/server/client';
import { failure, ok } from '@/test/api-results';

import PoetPage from './[slug].astro';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const POET = { name: 'قبيصة', slug: 'oNbs', era: { name: 'جاهلي', slug: 'jahili' }, poemsCount: 2 };
const POEMS = {
  data: [
    {
      title: 'لم أر خيلا',
      slug: 'kdmy',
      poet: { name: 'قبيصة', slug: 'oNbs', hasAvatar: false, isAnonymous: false },
      meter: { name: 'الطويل', slug: 'altawil' },
    },
  ],
  pagination: { page: 1, pageSize: 30, totalPages: 1, totalItems: 1 },
};
const FACETS = {
  data: {
    meters: [
      { name: 'الطويل', slug: 'altawil', poemsCount: 1 },
      { name: 'الوافر', slug: 'alwafir', poemsCount: 1 },
    ],
    rhymes: [],
    themes: [],
  },
};

function answer(facets: ReturnType<typeof ok> | ReturnType<typeof failure>) {
  get.mockImplementation((path: string) => {
    if (path === '/poets/{slug}') return ok({ data: POET });
    if (path === '/poems') return ok(POEMS);
    return facets;
  });
}

async function render(url: string) {
  const container = await AstroContainer.create({
    manifest: { trailingSlash: 'never' } as SSRManifest,
    renderers: await loadRenderers([getContainerRenderer()]),
  });
  return await container.renderToResponse(PoetPage, {
    request: new Request(url),
    params: { slug: 'oNbs' },
  });
}

beforeEach(() => {
  get.mockReset();
  vi.mocked(reportError).mockReset();
});

describe('GET /poets/[slug]', () => {
  it('shows the filters and caches the page when the facets load', async () => {
    answer(ok(FACETS));
    const response = await render('https://qafiyah.com/poets/oNbs');
    expect(response.headers.get('cache-control')).toBe('cache-html');
    expect(await response.text()).toContain('role="combobox"');
  });

  it('renders the poems without the filters, reports, and skips caching when the facets fail', async () => {
    answer(failure(500));
    const response = await render('https://qafiyah.com/poets/oNbs');
    expect(response.status).toBe(200);
    expect(response.headers.get('cache-control')).toBe('cache-none');
    const html = await response.text();
    expect(html).not.toContain('role="combobox"');
    expect(html).toContain('لم أر خيلا');
    expect(reportError).toHaveBeenCalledOnce();
  });

  it('server-renders a filtered URL with its poems, match count, and clear link, without indexing it', async () => {
    answer(ok(FACETS));
    const response = await render('https://qafiyah.com/poets/oNbs?meter=altawil');
    const html = await response.text();
    expect(html).toContain('لم أر خيلا');
    expect(html).toContain('text-text-subtle">قصيدة</p>');
    expect(html).toContain('مسح الكل');
    expect(html).toContain('noindex');
  });

  it('redirects reordered filters and an unknown parameter to the canonical URL without calling the API', async () => {
    const response = await render(
      'https://qafiyah.com/poets/oNbs?meter=alwafir&meter=altawil&meter=alwafir&utm_source=x'
    );
    expect(response.status).toBe(301);
    expect(response.headers.get('location')).toBe('/poets/oNbs?meter=altawil&meter=alwafir');
    expect(get).not.toHaveBeenCalled();
  });
});
