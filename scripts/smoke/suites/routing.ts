import { isStatus } from '../checks/status';
import { API, WEB } from '../target';

import type { Probe } from '../types';

const ALL = ['origin', 'stack', 'prod'] as const;

export const routingProbes: readonly Probe[] = [
  { url: `${WEB}/`, note: 'home', expect: 'ok', surfaces: ALL },
  { url: `${WEB}/about`, note: 'about page', expect: 'ok', surfaces: ALL },
  { url: `${WEB}/developers`, note: 'developers page', expect: 'ok', surfaces: ALL },
  { url: `${WEB}/login`, note: 'login page', expect: 'ok', surfaces: ALL },
  { url: `${WEB}/sponsorship`, note: 'sponsorship page', expect: 'ok', surfaces: ALL },
  {
    url: `${WEB}/does-not-exist`,
    note: 'unknown path maps to 404',
    expect: 'not-found',
    surfaces: ALL,
  },
  {
    url: `${WEB}/500`,
    note: 'server error page',
    checks: [isStatus(500)],
    surfaces: ['stack', 'prod'],
  },
  {
    url: `${API}/v1`,
    note: 'api root redirects to the docs',
    redirect: 'manual',
    checks: [isStatus(302)],
    surfaces: ALL,
  },
  { url: `${API}/v1/docs`, note: 'api reference HTML', expect: 'ok', surfaces: ALL },
  {
    url: `${API}/v1/go/x`,
    note: 'go link redirects',
    redirect: 'manual',
    checks: [isStatus(302)],
    surfaces: ALL,
  },
  {
    url: `${WEB}/poems/random`,
    note: 'random poem redirects',
    redirect: 'manual',
    checks: [isStatus(302)],
    surfaces: ALL,
  },
];
