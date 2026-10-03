import { beforeEach, describe, expect, it, vi } from 'vitest';

process.env['INTERNAL_API_URL'] = 'http://api.test';
process.env['INTERNAL_API_KEY'] = 'internal-key';

const fetchMock = vi.fn();
vi.stubGlobal('fetch', fetchMock);

function call(path: string, headers: Readonly<Record<string, string>> = {}, search = '?q=x') {
  return {
    params: { path },
    request: new Request(`https://qafiyah.com/api/v1/${path}`, { headers }),
    url: new URL(`https://qafiyah.com/api/v1/${path}${search}`),
  };
}

describe('the api proxy', () => {
  beforeEach(() => {
    fetchMock.mockReset();
    fetchMock.mockResolvedValue(
      new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
    );
  });

  it('refuses a path that is not on the allowlist', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    const response = await proxyRequest(call('poems/slugs'));
    expect(response.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('refuses a poem list that names no poet', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    const response = await proxyRequest(call('poems', {}, '?meter=altawil&meter=alkamil'));
    expect(response.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("forwards one poet's poem list", async () => {
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('poems', {}, '?poet=imHZ&meter=altawil'));
    const [target] = fetchMock.mock.calls[0] as [string];
    expect(target).toBe('http://api.test/v1/poems?poet=imHZ&meter=altawil');
  });

  it('forwards an allowlisted path with the internal key', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('search'));
    const [target, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(target).toBe('http://api.test/v1/search?q=x');
    expect((init.headers as Headers).get('x-api-key')).toBe('internal-key');
  });

  it('never forwards a client-supplied api key', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('search', { 'x-api-key': 'attacker-key' }));
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Headers).get('x-api-key')).toBe('internal-key');
  });

  it('returns 502 when the api is unreachable', async () => {
    fetchMock.mockRejectedValue(new Error('connect ECONNREFUSED'));
    const { proxyRequest } = await import('./proxy-handler');
    const response = await proxyRequest(call('search'));
    expect(response.status).toBe(502);
  });

  it('passes the query string through verbatim', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('search'));
    const [target] = fetchMock.mock.calls[0] as [string];
    expect(target).toBe('http://api.test/v1/search?q=x');
  });

  it('forwards an if-none-match header', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('search', { 'if-none-match': '"abc"' }));
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Headers).get('if-none-match')).toBe('"abc"');
  });

  it('forwards the visitor address nginx set as the address the api trusts', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('search', { 'x-real-ip': '2001:db8::7' }));
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Headers).get('cf-connecting-ip')).toBe('2001:db8::7');
  });

  it('forwards no visitor address when nginx set none', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('search'));
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Headers).has('cf-connecting-ip')).toBe(false);
  });

  it('never forwards a client-supplied cf-connecting-ip', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('search', { 'cf-connecting-ip': '198.51.100.9' }));
    await proxyRequest(
      call('search', { 'cf-connecting-ip': '198.51.100.9', 'x-real-ip': '2001:db8::7' })
    );
    const [[, alone], [, withNginx]] = fetchMock.mock.calls as [
      [string, RequestInit],
      [string, RequestInit],
    ];
    expect((alone.headers as Headers).has('cf-connecting-ip')).toBe(false);
    expect((withNginx.headers as Headers).get('cf-connecting-ip')).toBe('2001:db8::7');
  });

  it('copies only the safe response headers', async () => {
    fetchMock.mockResolvedValue(
      new Response('{}', {
        status: 200,
        headers: {
          'content-type': 'application/json',
          'cache-control': 'public',
          etag: '"e"',
          'x-upstream-secret': 'leak',
        },
      })
    );
    const { proxyRequest } = await import('./proxy-handler');
    const response = await proxyRequest(call('poems/random'));
    expect(response.headers.get('content-type')).toBe('application/json');
    expect(response.headers.get('cache-control')).toBe('public');
    expect(response.headers.get('etag')).toBe('"e"');
    expect(response.headers.get('x-upstream-secret')).toBeNull();
  });

  it('marks a successful search public, since the proxy strips the per-caller headers', async () => {
    const readPolicy = 'max-age=300, stale-while-revalidate=86400';
    const { proxyRequest } = await import('./proxy-handler');
    for (const status of [200, 304]) {
      fetchMock.mockResolvedValue(
        new Response(status === 304 ? null : '{}', {
          status,
          headers: { 'cache-control': `private, ${readPolicy}` },
        })
      );
      const response = await proxyRequest(call('search'));
      expect(response.headers.get('cache-control')).toBe(`public, ${readPolicy}`);
    }
  });

  it('leaves a failed search and the random poem uncacheable', async () => {
    const { proxyRequest } = await import('./proxy-handler');
    for (const status of [429, 503]) {
      fetchMock.mockResolvedValue(
        new Response('x', { status, headers: { 'cache-control': 'no-store' } })
      );
      expect((await proxyRequest(call('search'))).headers.get('cache-control')).toBe('no-store');
    }
    fetchMock.mockResolvedValue(
      new Response('abcd', { status: 200, headers: { 'cache-control': 'no-store' } })
    );
    const random = await proxyRequest(call('poems/random'));
    expect(random.headers.get('cache-control')).toBe('no-store');
  });

  it('returns a 304 with an empty body', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 304 }));
    const { proxyRequest } = await import('./proxy-handler');
    const response = await proxyRequest(call('search'));
    expect(response.status).toBe(304);
    expect(await response.text()).toBe('');
  });

  it('passes a 429 and a 5xx status through with when to retry', async () => {
    fetchMock.mockResolvedValue(
      new Response('x', { status: 429, headers: { 'retry-after': '30' } })
    );
    const { proxyRequest } = await import('./proxy-handler');
    const limited = await proxyRequest(call('search'));
    expect(limited.status).toBe(429);
    expect(limited.headers.get('retry-after')).toBe('30');
    fetchMock.mockResolvedValue(
      new Response('x', { status: 503, headers: { 'retry-after': '2' } })
    );
    const unavailable = await proxyRequest(call('search'));
    expect(unavailable.status).toBe(503);
    expect(unavailable.headers.get('retry-after')).toBe('2');
  });
});

describe('the api proxy with no internal key', () => {
  it('sends no api key header when the internal key is empty', async () => {
    vi.resetModules();
    process.env['INTERNAL_API_KEY'] = '';
    const { proxyRequest } = await import('./proxy-handler');
    await proxyRequest(call('search'));
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Headers).get('x-api-key')).toBeNull();
    vi.resetModules();
    process.env['INTERNAL_API_KEY'] = 'internal-key';
  });
});
