import * as v from 'valibot';

import { DEV_API_PORT } from '@qafiyah/config';

const urlSchema = v.pipe(v.string(), v.url());

export function resolveInternalApiUrl(raw: string | undefined): string {
  const candidate = raw ?? `http://localhost:${DEV_API_PORT}`;
  const parsed = v.safeParse(urlSchema, candidate);
  if (!parsed.success) {
    throw new Error(`INTERNAL_API_URL is not a valid URL: ${JSON.stringify(candidate)}`);
  }
  return parsed.output;
}

export const INTERNAL_API_URL = resolveInternalApiUrl(process.env['INTERNAL_API_URL']);

export const INTERNAL_API_KEY = process.env['INTERNAL_API_KEY'] ?? '';

export const OAUTH_GOOGLE_CLIENT_ID = process.env['OAUTH_GOOGLE_CLIENT_ID'] ?? '';
export const OAUTH_GOOGLE_CLIENT_SECRET = process.env['OAUTH_GOOGLE_CLIENT_SECRET'] ?? '';
export const OAUTH_GITHUB_CLIENT_ID = process.env['OAUTH_GITHUB_CLIENT_ID'] ?? '';
export const OAUTH_GITHUB_CLIENT_SECRET = process.env['OAUTH_GITHUB_CLIENT_SECRET'] ?? '';
export const SESSION_STATE_SECRET = process.env['SESSION_STATE_SECRET'] ?? '';
export const OTLP_METRICS_ENDPOINT = process.env['OTEL_EXPORTER_OTLP_METRICS_ENDPOINT'] ?? '';

export function providerConfigured(provider: 'google' | 'github'): boolean {
  return provider === 'google'
    ? OAUTH_GOOGLE_CLIENT_ID !== '' && OAUTH_GOOGLE_CLIENT_SECRET !== ''
    : OAUTH_GITHUB_CLIENT_ID !== '' && OAUTH_GITHUB_CLIENT_SECRET !== '';
}

export function clientIdFor(provider: 'google' | 'github'): string {
  return provider === 'google' ? OAUTH_GOOGLE_CLIENT_ID : OAUTH_GITHUB_CLIENT_ID;
}
