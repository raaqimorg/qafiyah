import { describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/server/client', () => ({ apiServer: { GET: vi.fn() } }));

import { apiServer } from '@/lib/server/client';
import { fakeContext } from '@/test/context';

import { GET } from './sitemap-index.xml';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

describe('GET /sitemap-index.xml', () => {
  it('lists the root, poets, and taxonomies sitemaps plus one shard per page of poems', async () => {
    get.mockResolvedValue({
      data: { data: { total: 45001 } },
      response: new Response(null, { status: 200 }),
    });
    const response = await GET(fakeContext());
    expect(response.status).toBe(200);
    const text = await response.text();
    expect(text).toContain('<sitemapindex');
    expect(text).toContain('/sitemap/root.xml');
    expect(text).toContain('/sitemap/poems/2.xml');
    expect(text).toContain('/sitemap/poets.xml');
    expect(text).toContain('/sitemap/taxonomies.xml');
  });
});
