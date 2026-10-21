import {
  DEV_API_PORT,
  DEV_EDGE_PORT,
  DEV_GRAFANA_PORT,
  DEV_MCP_PORT,
  DEV_WEB_PORT,
  PROD_API_URL,
  PROD_DOMAIN,
  PROD_SITE_URL,
} from '@qafiyah/config';

export type SurfaceName = 'origin' | 'stack' | 'prod';

export type Host = 'web' | 'api' | 'mcp';

export const STACK_API_KEY_FULL = 'dev-stack-full';
export const STACK_API_KEY_INTERNAL = 'dev-stack-internal';
export const STACK_API_KEY_MCP = 'dev-stack-mcp';
export const STACK_SESSION_STATE_SECRET = 'dev-stack-session-secret';
export const STACK_GRAFANA_PASSWORD = 'dev-stack-grafana';

export type Surface = {
  readonly name: SurfaceName;
  readonly web: string;
  readonly api: string;
  readonly mcp: string;
  readonly edgePort: number;
  readonly grafana: string | null;
  readonly manageServer: boolean;
  readonly prodOnly: boolean;
};

const HOST_HEADERS: Readonly<Record<Host, string>> = {
  web: PROD_DOMAIN,
  api: `api.${PROD_DOMAIN}`,
  mcp: `mcp.${PROD_DOMAIN}`,
};

export function hostHeader(host: Host): string {
  return HOST_HEADERS[host];
}

export function resolveSurface(): Surface {
  const positional = process.argv[2];
  const raw = (positional ?? process.env['SMOKE_TARGET'] ?? 'origin').toLowerCase();
  const requested = raw.startsWith('--') ? raw.slice(2) : raw;

  if (requested === 'origin' || requested === 'dev' || requested === 'local') {
    return {
      name: 'origin',
      web: `http://localhost:${DEV_WEB_PORT}`,
      api: `http://localhost:${DEV_API_PORT}`,
      mcp: `http://localhost:${DEV_MCP_PORT}`,
      edgePort: DEV_EDGE_PORT,
      grafana: null,
      manageServer: true,
      prodOnly: false,
    };
  }
  if (requested === 'stack') {
    return {
      name: 'stack',
      web: `http://localhost:${DEV_EDGE_PORT}`,
      api: `http://localhost:${DEV_EDGE_PORT}`,
      mcp: `http://localhost:${DEV_EDGE_PORT}`,
      edgePort: DEV_EDGE_PORT,
      grafana: `http://localhost:${DEV_GRAFANA_PORT}`,
      manageServer: true,
      prodOnly: false,
    };
  }
  if (requested === 'prod' || requested === 'production') {
    return {
      name: 'prod',
      web: PROD_SITE_URL,
      api: PROD_API_URL,
      mcp: `https://mcp.${PROD_DOMAIN}`,
      edgePort: 443,
      grafana: null,
      manageServer: false,
      prodOnly: true,
    };
  }
  console.error(`Unknown smoke surface "${requested}", expected "origin", "stack", or "prod".`);
  process.exit(2);
}
