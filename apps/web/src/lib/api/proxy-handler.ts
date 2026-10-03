import { resolveProxyPath } from '@/lib/api/proxy-allowlist';
import { CACHE_SEARCH } from '@/lib/server/cache';
import { INTERNAL_API_KEY, INTERNAL_API_URL } from '@/lib/server/env';
import {
  API_KEY_HEADER,
  API_V1_PREFIX,
  CF_CONNECTING_IP_HEADER,
  VISITOR_ADDRESS_HEADER,
} from '@qafiyah/config';

const PROXY_TIMEOUT_MS = 8000;
const FORWARDED_RESPONSE_HEADERS = ['content-type', 'cache-control', 'etag'] as const;

export type ProxyContext = {
  readonly params: Readonly<Record<string, string | undefined>>;
  readonly request: Request;
  readonly url: URL;
};

export async function proxyRequest({ params, request, url }: ProxyContext): Promise<Response> {
  const path = resolveProxyPath(params['path'], url.searchParams);
  if (path === undefined) {
    return new Response(null, { status: 404, headers: { 'Cache-Control': 'no-store' } });
  }

  const headers = new Headers();
  if (INTERNAL_API_KEY) headers.set(API_KEY_HEADER, INTERNAL_API_KEY);
  const ifNoneMatch = request.headers.get('if-none-match');
  if (ifNoneMatch !== null) headers.set('if-none-match', ifNoneMatch);
  const visitor = request.headers.get(VISITOR_ADDRESS_HEADER);
  if (visitor !== null) headers.set(CF_CONNECTING_IP_HEADER, visitor);

  const target = `${INTERNAL_API_URL}${API_V1_PREFIX}/${path}${url.search}`;
  let upstream: Response;
  try {
    upstream = await fetch(target, { headers, signal: AbortSignal.timeout(PROXY_TIMEOUT_MS) });
  } catch {
    return new Response(null, { status: 502, headers: { 'Cache-Control': 'no-store' } });
  }

  const out = new Headers();
  for (const name of FORWARDED_RESPONSE_HEADERS) {
    const value = upstream.headers.get(name);
    if (value !== null) out.set(name, value);
  }
  if (path === 'search' && (upstream.ok || upstream.status === 304)) {
    out.set('cache-control', CACHE_SEARCH);
  }
  if (upstream.status === 304) return new Response(null, { status: 304, headers: out });
  return new Response(upstream.body, { status: upstream.status, headers: out });
}
