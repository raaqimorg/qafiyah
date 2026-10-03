const PROXIED_PATHS = ['search', 'poems/random', 'poems', 'poems/facets'] as const;

type ProxiedPath = (typeof PROXIED_PATHS)[number];

export function normalizeProxyPath(raw: string | undefined): string {
  return (raw ?? '').replace(/^\/+/u, '').replace(/\/+$/u, '');
}

export function resolveProxyPath(raw: string | undefined): ProxiedPath | undefined {
  const normalized = normalizeProxyPath(raw);
  return PROXIED_PATHS.find((candidate) => candidate === normalized);
}
