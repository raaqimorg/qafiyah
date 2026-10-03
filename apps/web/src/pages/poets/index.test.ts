import { experimental_AstroContainer as AstroContainer } from 'astro/container';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { SSRManifest } from 'astro';

vi.mock('@/lib/server/client', () => ({ apiServer: { GET: vi.fn() } }));
vi.mock('@/lib/server/cache', () => ({ CACHE_HTML: 'cache-html', CACHE_NONE: 'cache-none' }));
vi.mock('@/lib/observability/report-error', () => ({ reportError: vi.fn() }));

import { apiServer } from '@/lib/server/client';
import { failure } from '@/test/api-results';

import PoetsPage from './index.astro';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

async function render(url: string) {
  const container = await AstroContainer.create({
    manifest: { trailingSlash: 'never' } as SSRManifest,
  });
  return await container.renderToResponse(PoetsPage, { request: new Request(url) });
}

beforeEach(() => {
  get.mockReset();
  get.mockImplementation(() => failure(404));
});

describe('GET /poets', () => {
  it.each([
    ['/poets?q=%D8%AD%D8%A8&utm_source=x', '/poets?q=%D8%AD%D8%A8'],
    ['/poets?page=02', '/poets?page=2'],
    ['/poets?page=2&era=abbasi', '/poets?era=abbasi&page=2'],
    ['/poets?era=%20abbasi%20', '/poets?era=abbasi'],
  ])('redirects %s to %s without calling the API', async (requested, canonical) => {
    const response = await render(`https://qafiyah.com${requested}`);
    expect(response.status).toBe(301);
    expect(response.headers.get('location')).toBe(canonical);
    expect(get).not.toHaveBeenCalled();
  });

  it('sends the canonical URL itself on to the API', async () => {
    await render('https://qafiyah.com/poets?era=abbasi&page=2').catch(() => undefined);
    expect(get).toHaveBeenCalledWith('/poets', expect.anything());
  });
});
