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

  it('keeps archive crawlers off the sponsorship page but lets search engines read its noindex', async () => {
    const text = await (await GET(fakeContext())).text();
    const groups = text.split('\n\n');
    const everyone = groups.find((group) => group.startsWith('User-agent: *'));
    const archives = groups.find((group) => group.startsWith('User-agent: archive.org_bot'));
    expect(everyone).toBeDefined();
    expect(everyone).not.toContain('/sponsorship');
    expect(archives).toContain('User-agent: archive.org_bot');
    expect(archives).toContain('User-agent: ia_archiver');
    expect(archives).toContain('Disallow: /sponsorship');
    expect(archives).toContain('Disallow: /poems/random');
  });
});
