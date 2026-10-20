import { isHealthy, isNoStore, isStatus } from '../checks/status';
import { WEB } from '../target';

import type { Probe } from '../types';

const ALL = ['origin', 'stack', 'prod'] as const;
const FORM_CONTENT_TYPE = 'application/x-www-form-urlencoded';

export const authProbes: readonly Probe[] = [
  {
    url: `${WEB}/login`,
    note: 'login is 200 and uncacheable',
    expect: 'ok',
    checks: [isNoStore],
    surfaces: ALL,
  },
  {
    url: `${WEB}/account`,
    note: 'account redirects to login',
    redirect: 'manual',
    checks: [isStatus(302)],
    surfaces: ALL,
  },
  {
    url: `${WEB}/api/me`,
    note: 'anonymous me is 204',
    checks: [isStatus(204), isNoStore],
    surfaces: ALL,
  },
  {
    url: `${WEB}/auth/google`,
    note: 'google 404 when unconfigured, 302 when configured',
    redirect: 'manual',
    checks: [isHealthy],
    surfaces: ['stack'],
  },
  {
    url: `${WEB}/auth/callback/google`,
    note: 'callback without state redirects',
    redirect: 'manual',
    checks: [isStatus(302)],
    surfaces: ALL,
  },
  {
    url: `${WEB}/auth/logout`,
    note: 'logout from another site is rejected',
    method: 'POST',
    headers: { Origin: 'https://example.com', 'Content-Type': FORM_CONTENT_TYPE },
    checks: [isStatus(403)],
    surfaces: ALL,
  },
  {
    url: `${WEB}/auth/logout`,
    note: 'logout from the site itself is accepted',
    method: 'POST',
    headers: { Origin: new URL(WEB).origin, 'Content-Type': FORM_CONTENT_TYPE },
    redirect: 'manual',
    checks: [isStatus(302)],
    surfaces: ['origin', 'prod'],
  },
];
