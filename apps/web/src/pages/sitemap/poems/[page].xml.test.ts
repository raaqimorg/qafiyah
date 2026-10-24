import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/server/client', () => ({ apiServer: { GET: vi.fn() } }));

import { apiServer } from '@/lib/server/client';
import { fakeContext } from '@/test/context';

import { GET } from './[page].xml';

const get = apiServer.GET as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => {
  get.mockReset();
});

describe('GET /sitemap/poems/[page].xml', () => {
  it('returns a well-formed urlset of absolute poem urls', async () => {
    get.mockResolvedValue({
      data: { data: ['aBcD', 'eFgH'], total: 2, next: null, prev: null },
      response: new Response(null, { status: 200 }),
    });
    const response = await GET(fakeContext({ params: { page: '1' } }));
    expect(response.status).toBe(200);
    expect(response.headers.get('content-type')).toBe('application/xml; charset=utf-8');
    const text = await response.text();
    expect(text).toContain('<urlset');
    expect(text).toContain('<loc>http://localhost:4321/poems/aBcD</loc>');
    expect(text).toContain('<loc>http://localhost:4321/poems/eFgH</loc>');
  });

  it('404s for a non-numeric page', async () => {
    const response = await GET(fakeContext({ params: { page: 'abc' } }));
    expect(response.status).toBe(404);
  });

  it('404s when the API reports the page as out of range (400 or 404), not 500', async () => {
    get.mockResolvedValue({
      data: undefined,
      error: { status: 400 },
      response: new Response(null, { status: 400 }),
    });
    const response = await GET(fakeContext({ params: { page: '99999' } }));
    expect(response.status).toBe(404);
  });

  it('follows next to reach a later shard', async () => {
    get
      .mockResolvedValueOnce({
        data: { data: ['aBcD'], total: 2, next: 'n', prev: null },
        response: new Response(null, { status: 200 }),
      })
      .mockResolvedValueOnce({
        data: { data: ['eFgH'], total: 2, next: null, prev: 'p' },
        response: new Response(null, { status: 200 }),
      });
    const response = await GET(fakeContext({ params: { page: '2' } }));
    expect(response.status).toBe(200);
    const text = await response.text();
    expect(text).toContain('<loc>http://localhost:4321/poems/eFgH</loc>');
    expect(text).not.toContain('aBcD');
    expect(get).toHaveBeenLastCalledWith('/poems/slugs', { params: { query: { cursor: 'n' } } });
  });

  it('404s past the last shard', async () => {
    get.mockResolvedValue({
      data: { data: ['aBcD'], total: 1, next: null, prev: null },
      response: new Response(null, { status: 200 }),
    });
    const response = await GET(fakeContext({ params: { page: '2' } }));
    expect(response.status).toBe(404);
  });

  it('404s when the shard is empty', async () => {
    get.mockResolvedValue({
      data: { data: [], total: 0, next: null, prev: null },
      response: new Response(null, { status: 200 }),
    });
    const response = await GET(fakeContext({ params: { page: '1' } }));
    expect(response.status).toBe(404);
  });
});
