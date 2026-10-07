import { getContainerRenderer } from '@astrojs/react/container-renderer';
import { experimental_AstroContainer as AstroContainer } from 'astro/container';
import { loadRenderers } from 'astro:container';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { SSRManifest } from 'astro';

vi.mock('@/lib/server/client', () => ({ apiServer: { GET: vi.fn() } }));
vi.mock('@/lib/server/cache', () => ({ CACHE_HTML: 'cache-html', CACHE_NONE: 'cache-none' }));
vi.mock('@/lib/observability/report-error', () => ({ reportError: vi.fn() }));

import { apiServer } from '@/lib/server/client';
import { ok } from '@/test/api-results';

import PoemPage from './[slug].astro';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

const SOURCE = 'رواية الأنباري، بضبط فيصل المنصور';
const POEM = {
  title: 'قفا نبك من ذكرى حبيب ومنزل',
  slug: 'rHUD',
  verses: [['قِفا نَبكِ مِن ذكرَى حبيبٍ وَّمنزِلِ', 'بِسَقطِ اللِّوى بينَ الدَّخولِ فحَوْمَلِ']],
  verseCount: 1,
  sample: 'قِفا نَبكِ',
  keywords: 'قفا,نبك',
  poet: { name: 'امرؤ القيس', slug: 'lVoU', hasAvatar: false, isAnonymous: false },
  era: { name: 'جاهلي', slug: 'jahili' },
  meter: { name: 'الطويل', slug: 'altawil' },
  theme: { name: 'غزل', slug: 'ghazal' },
  rhyme: { name: 'اللام', slug: 'l' },
  poemType: { name: 'عمودي', slug: 'amudi' },
  recensions: [],
  relatedPoems: [],
};

async function render(url: string) {
  const container = await AstroContainer.create({
    manifest: { trailingSlash: 'never' } as SSRManifest,
    renderers: await loadRenderers([getContainerRenderer()]),
  });
  return await container.renderToResponse(PoemPage, {
    request: new Request(url),
    params: { slug: 'rHUD' },
  });
}

beforeEach(() => {
  get.mockReset();
});

describe('GET /poems/[slug]', () => {
  it('shows the source of a reading under the breadcrumbs', async () => {
    get.mockImplementation(() => ok({ data: { ...POEM, source: SOURCE } }));
    const html = await (await render('https://qafiyah.com/poems/rHUD')).text();
    expect(html).toContain(SOURCE);
  });

  it('shows no source line when the reading has none', async () => {
    get.mockImplementation(() => ok({ data: POEM }));
    const response = await render('https://qafiyah.com/poems/rHUD');
    const html = await response.text();
    expect(response.status).toBe(200);
    expect(html).toContain('قفا نبك من ذكرى حبيب ومنزل');
    expect(html).not.toContain(SOURCE);
  });
});
