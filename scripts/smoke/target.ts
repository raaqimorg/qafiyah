import { API_KEY_HEADER, API_V1_PREFIX } from '@qafiyah/config';

import {
  hostHeader,
  resolveSurface,
  STACK_API_KEY_FULL,
  STACK_API_KEY_INTERNAL,
  STACK_GRAFANA_PASSWORD,
  STACK_SESSION_STATE_SECRET,
  type Surface,
} from './surfaces';

export {
  STACK_API_KEY_FULL,
  STACK_API_KEY_INTERNAL,
  STACK_GRAFANA_PASSWORD,
  STACK_SESSION_STATE_SECRET,
};

export const SURFACE: Surface = await resolveSurface();
export const TARGET = { name: SURFACE.name, manageServer: SURFACE.manageServer } as const;

export const WEB = SURFACE.web;
export const API = SURFACE.api;
export const SEARCH = `${API}${API_V1_PREFIX}/search`;

function resolveApiKey(): string | undefined {
  const fromProcess = process.env['SMOKE_API_KEY'] ?? process.env['API_KEY_FULL'];
  if (fromProcess) return fromProcess;
  if (SURFACE.name === 'stack') return STACK_API_KEY_FULL;
  return undefined;
}

export const API_KEY = resolveApiKey();

function parseConcurrency(fallback: number): number {
  const raw = process.env['SMOKE_CONCURRENCY'];
  const n = raw ? Number(raw) : Number.NaN;
  return Number.isInteger(n) && n > 0 ? Math.min(n, 64) : fallback;
}

export const CONCURRENCY = parseConcurrency(SURFACE.name === 'prod' ? 1 : 8);

export const POEMS_LIST = `${API}${API_V1_PREFIX}/poems`;
export const RANDOM_POEM = `${API}${API_V1_PREFIX}/poems/random`;
export const POEM_DETAIL = (slug: string): string => `${API}${API_V1_PREFIX}/poems/${slug}`;
export const OPENAPI_SPEC = `${API}${API_V1_PREFIX}/openapi.json`;

export function authHeaders(url: string): Readonly<Record<string, string>> | undefined {
  if (API_KEY && url.startsWith(API)) return { [API_KEY_HEADER]: API_KEY };
  return undefined;
}

export { hostHeader };
