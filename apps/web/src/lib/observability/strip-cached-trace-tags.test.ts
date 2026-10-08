import { describe, expect, it } from 'vitest';

import { stripCachedTraceTags } from './strip-cached-trace-tags';

import type { APIContext } from 'astro';

const TRACE_TAGS =
  '<meta name="sentry-trace" content="bcf67c94869d4e1da232e8605fb434a9-8ebd0868964a5f4f-1"/>' +
  '<meta name="baggage" content="sentry-environment=production,sentry-release=a3a4a17,sentry-sampled=true"/>';
const PAGE = `<html><head>${TRACE_TAGS}<meta name="sentry-route-name" content="%2F"/><title>t</title></head><body></body></html>`;

async function run(html: string, headers: Record<string, string>): Promise<Response> {
  const response = new Response(html, { status: 200, headers });
  return await stripCachedTraceTags({} as APIContext, () => Promise.resolve(response));
}

describe('stripCachedTraceTags', () => {
  it('removes the trace tags from a page that caches may store', async () => {
    const response = await run(PAGE, {
      'content-type': 'text/html',
      'cache-control': 'public, max-age=60, s-maxage=86400, stale-while-revalidate=600',
    });
    const html = await response.text();
    expect(html).not.toContain('sentry-trace');
    expect(html).not.toContain('name="baggage"');
    expect(html).toContain('sentry-route-name');
    expect(response.headers.get('cache-control')).toContain('s-maxage=86400');
  });

  it('keeps the trace tags on a page that no cache stores', async () => {
    const response = await run(PAGE, { 'content-type': 'text/html', 'cache-control': 'no-store' });
    expect(await response.text()).toContain('sentry-trace');
  });

  it('leaves responses that are not HTML untouched', async () => {
    const body = '{"sentry-trace":"x"}';
    const response = await run(body, {
      'content-type': 'application/json',
      'cache-control': 'public',
    });
    expect(await response.text()).toBe(body);
  });
});
