import {
  DEV_API_PORT,
  DEV_EDGE_PORT,
  DEV_GRAFANA_PORT,
  DEV_WEB_PORT,
  PROD_API_URL,
  PROD_DOMAIN,
  PROD_SITE_URL,
} from '@qafiyah/config';

import { resolveWorktreeIdentity } from '../dev/worktree';

export type SurfaceName = 'origin' | 'stack' | 'prod';

export type Host = 'web' | 'api';

export const STACK_API_KEY_FULL = 'dev-stack-full';
export const STACK_API_KEY_INTERNAL = 'dev-stack-internal';
export const STACK_SESSION_STATE_SECRET = 'dev-stack-session-secret';
export const STACK_GRAFANA_PASSWORD = 'dev-stack-grafana';

export type Surface = {
  readonly name: SurfaceName;
  readonly web: string;
  readonly api: string;
  readonly edgePort: number;
  readonly grafana: string | null;
  readonly manageServer: boolean;
  readonly prodOnly: boolean;
};

export function hostHeader(host: Host): string {
  return host === 'web' ? PROD_DOMAIN : `api.${PROD_DOMAIN}`;
}

export async function resolveSurface(): Promise<Surface> {
  const positional = process.argv.slice(2).find((arg) => arg !== '--worktree');
  const raw = (positional ?? process.env['SMOKE_TARGET'] ?? 'origin').toLowerCase();
  const requested = raw.startsWith('--') ? raw.slice(2) : raw;

  let offset = 0;
  if (process.argv.includes('--worktree')) {
    const identity = await resolveWorktreeIdentity();
    if (!identity.isWorktree) {
      console.error(
        '--worktree passed but this is the primary checkout, nothing to isolate against'
      );
      process.exit(2);
    }
    offset = identity.offset;
  }

  if (requested === 'origin' || requested === 'dev' || requested === 'local') {
    return {
      name: 'origin',
      web: `http://localhost:${DEV_WEB_PORT + offset}`,
      api: `http://localhost:${DEV_API_PORT + offset}`,
      edgePort: DEV_EDGE_PORT + offset,
      grafana: null,
      manageServer: true,
      prodOnly: false,
    };
  }
  if (requested === 'stack') {
    return {
      name: 'stack',
      web: `http://localhost:${DEV_EDGE_PORT + offset}`,
      api: `http://localhost:${DEV_EDGE_PORT + offset}`,
      edgePort: DEV_EDGE_PORT + offset,
      grafana: `http://localhost:${DEV_GRAFANA_PORT + offset}`,
      manageServer: true,
      prodOnly: false,
    };
  }
  if (requested === 'prod' || requested === 'production') {
    return {
      name: 'prod',
      web: PROD_SITE_URL,
      api: PROD_API_URL,
      edgePort: 443,
      grafana: null,
      manageServer: false,
      prodOnly: true,
    };
  }
  console.error(`Unknown smoke surface "${requested}", expected "origin", "stack", or "prod".`);
  process.exit(2);
}
