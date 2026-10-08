import * as Sentry from '@sentry/astro';

Sentry.init({
  dsn: 'https://acbece4802d36f6e2904c8d673635c0d@o4511571113148416.ingest.us.sentry.io/4511594177560576',
  enabled: Boolean(import.meta.env.PUBLIC_SENTRY_RELEASE),
  environment: import.meta.env.DEV ? 'development' : 'production',
  tracesSampleRate: 0.1,
  dataCollection: {
    cookies: false,
    urlQueryParams: false,
    frameContextLines: 7,
  },
});
