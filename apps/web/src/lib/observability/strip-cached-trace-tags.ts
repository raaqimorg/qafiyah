import type { MiddlewareHandler } from 'astro';

const TRACE_META_TAGS = /<meta name="(?:sentry-trace|baggage)" content="[^"]*"\/>/g;

function isCachedHtml(response: Response): boolean {
  const contentType = response.headers.get('content-type') ?? '';
  const cacheControl = response.headers.get('cache-control') ?? '';
  return contentType.startsWith('text/html') && cacheControl.startsWith('public');
}

export const stripCachedTraceTags = (async (_context, next) => {
  const response = await next();
  if (!isCachedHtml(response)) return response;
  const html = await response.text();
  const headers = new Headers(response.headers);
  headers.delete('content-length');
  return new Response(html.replace(TRACE_META_TAGS, ''), {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}) satisfies MiddlewareHandler;
