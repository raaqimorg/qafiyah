import * as Sentry from '@sentry/astro';
import { posthog } from 'posthog-js';

const LOW_SIGNAL_REJECTION_MAX_LENGTH = 3;

function isLowSignalRejection(event) {
  const values = event.exception?.values;
  if (!values || values.length === 0) {
    return false;
  }
  return values.every((value) => {
    const isUnhandledRejection = value.mechanism?.type === 'onunhandledrejection';
    const hasNoStack = (value.stacktrace?.frames?.length ?? 0) === 0;
    const message = value.value?.trim() ?? '';
    return isUnhandledRejection && hasNoStack && message.length <= LOW_SIGNAL_REJECTION_MAX_LENGTH;
  });
}

Sentry.init({
  dsn: 'https://acbece4802d36f6e2904c8d673635c0d@o4511571113148416.ingest.us.sentry.io/4511594177560576',
  enabled: Boolean(import.meta.env.PUBLIC_SENTRY_RELEASE),
  environment: import.meta.env.DEV ? 'development' : 'production',
  tracesSampleRate: 0.1,
  integrations: [
    posthog.sentryIntegration({
      organization: 'qafiyah',
      projectId: 4511594177560576,
      sendExceptionsToPostHog: false,
    }),
  ],
  dataCollection: {
    cookies: false,
    urlQueryParams: false,
    frameContextLines: 7,
  },
  ignoreErrors: [
    /__firefox__/,
    /__gCrWeb/,
    /window\.webkit\.messageHandlers/,
    /currentInset/,
    /Unexpected token '<'/,
    /Importing a module script failed/,
    /Failed to fetch dynamically imported module/,
    /error loading dynamically imported module/,
    /module script failed to load/,
  ],
  beforeSend: (event) => {
    if (event.request?.url) {
      event.request.url = event.request.url.split('?')[0];
    }
    return isLowSignalRejection(event) ? null : event;
  },
});
