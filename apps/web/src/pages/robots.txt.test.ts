import { describe, expect, it } from 'vitest';

import { fakeContext } from '@/test/context';

import { GET } from './robots.txt';

describe('GET /robots.txt', () => {
  it('serves text with no leftover placeholder', async () => {
    const response = await GET(fakeContext());
    expect(response.headers.get('content-type')).toBe('text/plain; charset=utf-8');
    expect(response.headers.get('cache-control')).toContain('public');
    const text = await response.text();
    expect(text).not.toContain('{SITE}');
    expect(text).not.toContain('{API}');
    expect(text).toContain('Sitemap: http://localhost:4321/sitemap-index.xml');
  });

  it('keeps crawlers off the random poem redirect', async () => {
    const response = await GET(fakeContext());
    expect(await response.text()).toContain('Disallow: /poems/random');
  });
});
