import { err, ok, type Result } from 'neverthrow';

import { isStatus } from '../checks/status';
import { MCP } from '../target';

import type { Check, Probe } from '../types';

const ALL = ['origin', 'stack', 'prod'] as const;

const RPC_HEADERS = {
  'Content-Type': 'application/json',
  Accept: 'application/json, text/event-stream',
  'MCP-Protocol-Version': '2025-06-18',
} as const;

const rpc = (method: string, params: Readonly<Record<string, unknown>>): string =>
  JSON.stringify({ jsonrpc: '2.0', id: 1, method, params });

const INITIALIZE = rpc('initialize', {
  protocolVersion: '2025-06-18',
  capabilities: {},
  clientInfo: { name: 'qafiyah-smoke', version: '1' },
});
const TOOLS_LIST = rpc('tools/list', {});
const SEARCH = rpc('tools/call', { name: 'search', arguments: { q: 'سلام', types: ['poems'] } });

type Json = Readonly<Record<string, unknown>>;

const isObject = (value: unknown): value is Json =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

function resultOf(body: string): Result<Json, string> {
  try {
    const parsed: unknown = JSON.parse(body);
    if (!isObject(parsed)) return err('the body is not a JSON object');
    const result = parsed['result'];
    return isObject(result) ? ok(result) : err('the response has no `result` object');
  } catch {
    return err('the body is not JSON');
  }
}

const namesTheServer: Check = {
  name: 'initialize names the qafiyah server',
  run: (body) =>
    resultOf(body).andThen((result) => {
      const info = result['serverInfo'];
      return isObject(info) && info['name'] === 'qafiyah'
        ? ok(undefined)
        : err('serverInfo.name is not qafiyah');
    }),
};

const listsNineReadOnlyTools: Check = {
  name: 'tools/list has nine read-only tools',
  run: (body) =>
    resultOf(body).andThen((result) => {
      const tools = result['tools'];
      if (!Array.isArray(tools) || tools.length !== 9) return err('there are not nine tools');
      const writable = tools.filter((tool: unknown) => {
        const annotations = isObject(tool) ? tool['annotations'] : undefined;
        return !isObject(annotations) || annotations['readOnlyHint'] !== true;
      });
      return writable.length === 0 ? ok(undefined) : err('a tool is not marked read-only');
    }),
};

const searchSucceeds: Check = {
  name: 'a search answers without a tool error',
  run: (body) =>
    resultOf(body).andThen((result) =>
      result['isError'] === false ? ok(undefined) : err('the search answered a tool error')
    ),
};

const post = (body: string, headers: Readonly<Record<string, string>> = {}) =>
  ({
    url: `${MCP}/mcp`,
    method: 'POST',
    body,
    headers: { ...RPC_HEADERS, ...headers },
    host: 'mcp',
    unkeyed: true,
  }) as const;

export const mcpProbes: readonly Probe[] = [
  {
    url: `${MCP}/healthz`,
    note: 'mcp healthz is 200',
    host: 'mcp',
    unkeyed: true,
    checks: [isStatus(200)],
    surfaces: ['origin'],
  },
  {
    ...post(INITIALIZE),
    note: 'mcp initialize answers the server info',
    checks: [isStatus(200), namesTheServer],
    surfaces: ALL,
  },
  {
    ...post(TOOLS_LIST),
    note: 'mcp tools/list answers nine read-only tools',
    checks: [isStatus(200), listsNineReadOnlyTools],
    surfaces: ALL,
  },
  {
    ...post(SEARCH),
    note: 'mcp search calls the API and answers a result',
    checks: [isStatus(200), searchSucceeds],
    surfaces: ALL,
  },
  {
    ...post(TOOLS_LIST, { Origin: 'https://evil.example' }),
    note: 'mcp refuses a request that carries an Origin',
    checks: [isStatus(403)],
    surfaces: ['origin', 'stack'],
  },
  {
    url: `${MCP}/mcp`,
    note: 'mcp answers GET with 405',
    host: 'mcp',
    unkeyed: true,
    headers: { Accept: RPC_HEADERS.Accept },
    checks: [isStatus(405)],
    surfaces: ALL,
  },
  {
    url: `${MCP}/other`,
    note: 'the mcp host answers 404 outside /mcp',
    host: 'mcp',
    unkeyed: true,
    checks: [isStatus(404)],
    surfaces: ['stack', 'prod'],
  },
];
