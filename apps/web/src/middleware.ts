import { hostname } from 'node:os';

import { requestDurationRecorder } from './lib/observability/request-metrics';
import { claimTiming, timeRequest } from './lib/observability/request-timing';
import { OTLP_METRICS_ENDPOINT } from './lib/server/env';

import type { MiddlewareHandler } from 'astro';

const record = requestDurationRecorder(OTLP_METRICS_ENDPOINT, hostname());

export const onRequest: MiddlewareHandler = (context, next) =>
  record === undefined || !claimTiming(context.locals)
    ? next()
    : timeRequest(record, { method: context.request.method, route: context.routePattern }, next);
