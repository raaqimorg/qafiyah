type QueryRule = (query: URLSearchParams) => boolean;

const anyQuery: QueryRule = () => true;

const isPoetKey = (key: string): boolean => key === 'poet' || key.startsWith('poet[');

const namesOnePoet: QueryRule = (query) =>
  [...query.keys()].filter((key) => isPoetKey(key)).length === 1;

const PROXIED_PATHS = {
  search: anyQuery,
  'poems/random': anyQuery,
  poems: namesOnePoet,
  'poems/facets': namesOnePoet,
} as const satisfies Record<string, QueryRule>;

type ProxiedPath = keyof typeof PROXIED_PATHS;

const isProxiedPath = (path: string): path is ProxiedPath => Object.hasOwn(PROXIED_PATHS, path);

export function normalizeProxyPath(raw: string | undefined): string {
  return (raw ?? '').replace(/^\/+/u, '').replace(/\/+$/u, '');
}

export function resolveProxyPath(
  raw: string | undefined,
  query: URLSearchParams
): ProxiedPath | undefined {
  const path = normalizeProxyPath(raw);
  return isProxiedPath(path) && PROXIED_PATHS[path](query) ? path : undefined;
}
