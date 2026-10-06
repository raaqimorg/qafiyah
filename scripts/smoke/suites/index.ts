import { httpShapeProbes } from '../probes/http-shape';
import { detailBoundaryProbes, pageProbes, poetsSsrProbes } from '../probes/pages';
import { poemNavProbes } from '../probes/poem-nav';
import { rateLimitProbes } from '../probes/rate-limit';
import {
  searchAbsentSlugProbes,
  searchArrayDecoderProbes,
  searchEncodingProbes,
  searchOkProbes,
  searchPageCoercionProbes,
  searchRejectProbes,
  searchSemanticEdgeProbes,
  searchUnicodeProbes,
} from '../probes/search';
import { securityProbes } from '../probes/security';
import { verseProbes } from '../probes/verses';

import { apiContractProbes } from './api-contract';
import { authProbes } from './auth';
import { cachingProbes } from './caching';
import { healthProbes } from './health';
import { inputSafetyProbes } from './input-safety';
import { proxyProbes } from './proxy';
import { routingProbes } from './routing';
import { securityHeaderProbes } from './security-headers';
import { seoProbes } from './seo';
import { sitemapProbes } from './sitemaps';
import { wellKnownProbes } from './well-known';

import type { Suite } from '../types';

export const SUITES: readonly Suite[] = [
  { name: 'routing', probes: routingProbes },
  { name: 'health', probes: healthProbes },
  { name: 'api-contract', probes: apiContractProbes },
  {
    name: 'search',
    probes: [
      ...searchOkProbes,
      ...searchSemanticEdgeProbes,
      ...searchUnicodeProbes,
      ...searchEncodingProbes,
      ...searchArrayDecoderProbes,
      ...searchPageCoercionProbes,
      ...searchAbsentSlugProbes,
      ...searchRejectProbes,
    ],
  },
  { name: 'pages', probes: [...pageProbes, ...poetsSsrProbes, ...detailBoundaryProbes] },
  { name: 'poem-nav', probes: poemNavProbes },
  { name: 'verses', probes: verseProbes },
  { name: 'proxy', probes: proxyProbes },
  { name: 'well-known', probes: wellKnownProbes },
  { name: 'sitemaps', probes: sitemapProbes },
  { name: 'seo', probes: seoProbes },
  { name: 'auth', probes: authProbes },
  { name: 'caching', probes: cachingProbes },
  { name: 'security-headers', probes: securityHeaderProbes },
  { name: 'input-safety', probes: inputSafetyProbes },
  { name: 'http-shape', probes: httpShapeProbes },
  { name: 'security', probes: securityProbes },
  { name: 'rate-limit', probes: rateLimitProbes, serial: true },
];
