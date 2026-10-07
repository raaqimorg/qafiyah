import { env } from '@/env';
import {
  DEV_WEB_PORT,
  POSTHOG_PROXY_URL,
  PROD_API_URL,
  PROD_DOMAIN,
  SUPPORT_EMAIL,
} from '@qafiyah/config';

export const REACT_QUERY_STALE_TIME_MS = 2 * 24 * 60 * 60 * 1000;
export const REACT_QUERY_GC_TIME_MS = 3 * 24 * 60 * 60 * 1000;
export const REACT_QUERY_RETRY_COUNT = 1;
export const SEARCH_NETWORK_RETRY_COUNT = 3;
export const SEARCH_RETRY_BASE_DELAY_MS = 500;
export const SEARCH_RETRY_MAX_DELAY_MS = 4000;

export const SSR_NETWORK_RETRY_COUNT = 2;
export const SSR_RETRY_BASE_DELAY_MS = 50;
export const SSR_RETRY_MAX_DELAY_MS = 300;
export const SSR_FETCH_TIMEOUT_MS = 10_000;

export const POSTHOG_KEY = 'phc_m925vSZaBCkXApwManQBMvCvtU6tHJVErNJiRr5ier8L';
export const POSTHOG_HOST = POSTHOG_PROXY_URL;
export const POSTHOG_UI_HOST = 'https://us.posthog.com';

export { SUPPORT_EMAIL };

const DEV_WEB_URL = `http://localhost:${DEV_WEB_PORT}`;

export const isDev = env.DEV;
export const API_URL = env.PUBLIC_API_URL ?? PROD_API_URL;

export const WEB_API_PROXY_PREFIX = '/api';
export const SITE_URL = isDev ? DEV_WEB_URL : `https://${PROD_DOMAIN}`;

export const THEME_STORAGE_KEY = 'qafiyah-theme';
export const MODULE_RELOAD_STORAGE_KEY = 'qafiyah-module-reload-at';
export const VIEWER_STORAGE_KEY = 'qafiyah-viewer';
export const VIEWER_HINT_COOKIE = 'qaf_viewer';
export const SESSION_MAX_AGE_SECONDS = 30 * 24 * 60 * 60;
export const MODULE_RELOAD_COOLDOWN_MS = 10000;
