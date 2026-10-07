import { headerIncludes, headerPresent } from '../checks/headers';
import { isStatus } from '../checks/status';
import { API, WEB } from '../target';

import type { Check, Probe } from '../types';

const STACK = ['stack', 'prod'] as const;

const securityHeaders: readonly Check[] = [
  headerPresent('content-security-policy'),
  headerPresent('strict-transport-security'),
  headerPresent('x-content-type-options'),
  headerPresent('x-frame-options'),
  headerPresent('referrer-policy'),
  headerPresent('cross-origin-opener-policy'),
];

export const securityHeaderProbes: readonly Probe[] = [
  {
    url: `${WEB}/`,
    note: 'web homepage carries the security headers',
    expect: 'ok',
    checks: securityHeaders,
    surfaces: STACK,
  },
  {
    url: `${WEB}/missing`,
    note: 'web error page carries the security headers',
    checks: [isStatus(404), ...securityHeaders],
    surfaces: STACK,
  },
  {
    url: `${API}/v1/poems`,
    note: 'api host carries a csp',
    expect: 'ok',
    checks: [headerPresent('content-security-policy')],
    surfaces: STACK,
  },
  {
    url: `${WEB}/?smoke=${crypto.randomUUID()}`,
    note: 'bad-bot user agent is blocked even where Cloudflare would serve a cached page',
    headers: { 'User-Agent': 'sqlmap' },
    checks: [isStatus(403)],
    surfaces: STACK,
  },
  {
    url: `${WEB}/?smoke=${crypto.randomUUID()}`,
    note: 'empty user agent is blocked even where Cloudflare would serve a cached page',
    headers: { 'User-Agent': '' },
    checks: [isStatus(403)],
    surfaces: STACK,
  },
  {
    url: `${API}/account`,
    note: 'account is 404 on the api vhost',
    host: 'api',
    checks: [isStatus(404)],
    surfaces: STACK,
  },
  {
    url: `${WEB}/`,
    note: 'gzip is served for text',
    headers: { 'Accept-Encoding': 'gzip' },
    expect: 'ok',
    checks: [headerIncludes('content-encoding', 'gzip')],
    surfaces: STACK,
  },
];
