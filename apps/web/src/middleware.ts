import { hostname } from 'node:os';

import { handleRequest } from '@sentry/astro';
import { sequence } from 'astro/middleware';

import { requestDurationRecorder } from './lib/observability/request-metrics';
import { claimTiming, timeRequest } from './lib/observability/request-timing';
import { stripCachedTraceTags } from './lib/observability/strip-cached-trace-tags';
import { OTLP_METRICS_ENDPOINT } from './lib/server/env';

import type { MiddlewareHandler } from 'astro';

const record = requestDurationRecorder(OTLP_METRICS_ENDPOINT, hostname());

const timeRequests: MiddlewareHandler = (context, next) =>
  record === undefined || !claimTiming(context.locals)
    ? next()
    : timeRequest(record, { method: context.request.method, route: context.routePattern }, next);

export const onRequest = sequence(stripCachedTraceTags, handleRequest(), timeRequests);
