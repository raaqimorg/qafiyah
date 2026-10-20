#!/usr/bin/env bun

import net from 'node:net';

import { err, ok, type Result } from 'neverthrow';

import { DEV_API_PORT, DEV_MCP_PORT, DEV_POSTGRES_PORT, DEV_WEB_PORT } from '@qafiyah/config';

const HOST = '127.0.0.1';

const PORTS = { api: DEV_API_PORT, web: DEV_WEB_PORT, mcp: DEV_MCP_PORT } as const;
const PORT_ENV_KEYS = { api: 'PORT', web: 'WEB_PORT', mcp: 'MCP_PORT' } as const;
type PortName = keyof typeof PORTS;

const isPortName = (value: string): value is PortName => value in PORTS;

function effectivePort(name: PortName): number {
  return Number(process.env[PORT_ENV_KEYS[name]] ?? PORTS[name]);
}

type ProbeError = {
  readonly kind: 'unknown_listen_failure';
  readonly host: string;
  readonly port: number;
  readonly code?: string;
  readonly message: string;
};

function probePort(port: number, host: string): Result<'busy' | 'free', ProbeError> {
  try {
    Bun.listen({
      hostname: host,
      port,
      socket: {
        data: () => undefined,
      },
    }).stop(true);
    return ok('free');
  } catch (cause) {
    const code =
      cause instanceof Error &&
      'code' in cause &&
      typeof (cause as { code?: unknown }).code === 'string'
        ? (cause as { code: string }).code
        : undefined;
    if (code === 'EADDRINUSE') return ok('busy');
    return err({
      kind: 'unknown_listen_failure',
      host,
      port,
      ...(code !== undefined && { code }),
      message: cause instanceof Error ? cause.message : String(cause),
    });
  }
}

function checkPortFree(port: number): number {
  for (const host of [HOST, '::1'] as const) {
    const result = probePort(port, host);
    if (result.isErr()) {
      console.warn(
        `[predev] note: could not probe ${host}:${port}: ${result.error.message}${result.error.code ? ` (${result.error.code})` : ''}; treating as free`
      );
      continue;
    }
    if (result.value === 'busy') {
      console.error(`\n[predev] port ${port} is already in use.`);
      console.error('[predev] another dev server is probably still running.');
      console.error('[predev] run `bun run clean` to clear orphans, then try again.\n');
      return 1;
    }
  }
  return 0;
}

function postgresReachable(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = new net.Socket();
    socket.setTimeout(1000);
    socket.connect(port, HOST, () => {
      socket.destroy();
      resolve(true);
    });
    socket.on('error', () => resolve(false));
    socket.on('timeout', () => {
      socket.destroy();
      resolve(false);
    });
  });
}

const requested = Bun.argv[2] ?? 'api';
if (!isPortName(requested)) {
  console.error(
    `[predev] usage: preflight.ts [${Object.keys(PORTS).join('|')}]  (got "${requested}")`
  );
  process.exit(2);
}

const portCode = checkPortFree(effectivePort(requested));
if (portCode !== 0) process.exit(portCode);

if (requested === 'api') {
  const postgresPort = Number(process.env['DEV_POSTGRES_PORT'] ?? DEV_POSTGRES_PORT);
  if (!(await postgresReachable(postgresPort))) {
    console.warn(`\n[predev] warn: Postgres is not reachable on ${HOST}:${postgresPort}.`);
    console.warn('[predev] warn: the qafiyah Docker container may not be running.');
    console.warn('[predev] warn: run `bun run db:up` to start it.\n');
  }
}
