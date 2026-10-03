import { afterEach, describe, expect, it, vi } from 'vitest';

process.env['INTERNAL_API_URL'] = 'http://api.test';
process.env['INTERNAL_API_KEY'] = 'internal-key';

import { fakeContext } from '@/test/context';
import { fakeFetch } from '@/test/fetch';

afterEach(() => {
  vi.unstubAllGlobals();
});

async function load() {
  return await import('./[...path]');
}

describe('GET /api/v1/[...path]', () => {
  it('proxies an allowlisted path and forwards only the safe response headers', async () => {
    const mock = fakeFetch({
      'GET /v1/search': () =>
        new Response('{}', {
          status: 200,
          headers: {
            'content-type': 'application/json',
            etag: '"x"',
            'x-upstream-secret': 'leak',
          },
        }),
    });
    const { GET } = await load();
    const response = await GET(
      fakeContext({
        url: 'https://qafiyah.com/api/v1/search?q=%D8%AD%D8%A8',
        params: { path: 'search' },
      })
    );
    expect(response.status).toBe(200);
    expect(response.headers.get('content-type')).toBe('application/json');
    expect(response.headers.get('etag')).toBe('"x"');
    expect(response.headers.get('x-upstream-secret')).toBeNull();
    expect(mock).toHaveBeenCalled();
  });

  it('returns 404 no-store for a blocked path without hitting the API', async () => {
    const mock = fakeFetch({});
    const { GET } = await load();
    const response = await GET(
      fakeContext({
        url: 'https://qafiyah.com/api/v1/poems/slugs',
        params: { path: 'poems/slugs' },
      })
    );
    expect(response.status).toBe(404);
    expect(response.headers.get('cache-control')).toBe('no-store');
    expect(mock).not.toHaveBeenCalled();
  });
});
