export const MAX_TWEET_LENGTH = 280;

export const SECONDS_PER_HOUR = 3600;

export const POEMS_PER_PAGE = 30;
export const SEARCH_POEMS_PER_PAGE = 20;

export const SITEMAP_POEMS_PER_SHARD = 45_000;
export const SITEMAP_POETS_PER_SHARD = 45_000;
export const SEARCH_POETS_PER_PAGE = 20;

export const ES_MAX_RESULT_WINDOW = 10_000;
export const SEARCH_POEMS_MAX_PAGE = Math.floor(ES_MAX_RESULT_WINDOW / SEARCH_POEMS_PER_PAGE);
export const SEARCH_POETS_MAX_PAGE = Math.floor(ES_MAX_RESULT_WINDOW / SEARCH_POETS_PER_PAGE);

export const POETS_LIST_MAX_RESULT_WINDOW = 50_000;
export const LIST_POETS_MAX_PAGE = Math.floor(POETS_LIST_MAX_RESULT_WINDOW / POEMS_PER_PAGE);

export const MAX_FILTER_SLUGS = 100;
export const MAX_QUERY_LENGTH = 50;

export const SEARCH_TYPE_VALUES = ['poems', 'poets'] as const;
export type SearchType = (typeof SEARCH_TYPE_VALUES)[number];

export const PROD_DOMAIN = 'qafiyah.com';
export const PROD_SITE_URL = `https://${PROD_DOMAIN}`;
export const PROD_API_URL = 'https://api.qafiyah.com';
export const POSTHOG_PROXY_URL = `https://ix.${PROD_DOMAIN}`;
export const TELEMETRY_PROXY_URL = `https://t.${PROD_DOMAIN}`;
export const CDN_URL = `https://cdn.${PROD_DOMAIN}`;

export const X_INTENT_TWEET_URL = 'https://x.com/intent/tweet';

const X_HANDLE = 'qafiyahx';
export const X_PROFILE_URL = `https://x.com/${X_HANDLE}`;

const TELEGRAM_HANDLE = 'qafiyahx';
export const TELEGRAM_URL = `https://t.me/${TELEGRAM_HANDLE}`;

const GITHUB_REPO = 'raaqimorg/qafiyah';
export const GITHUB_URL = `https://github.com/${GITHUB_REPO}`;
export const GITHUB_DB_DUMPS_URL = `${GITHUB_URL}/tree/main/data/db`;
export const GITHUB_AVATARS_URL = `${GITHUB_URL}/tree/main/data/avatars`;

export const RAAQIM_URL = 'https://raaqim.org';

export const SUPPORT_EMAIL = `issues@${PROD_DOMAIN}`;
export const CONTACT_EMAIL = `mail@${PROD_DOMAIN}`;
export const SECURITY_EMAIL = `security@${PROD_DOMAIN}`;
export const API_EMAIL = `api@${PROD_DOMAIN}`;

export const API_V1_PREFIX = '/v1';
export const API_RANDOM_POEM_PATH = `${API_V1_PREFIX}/poems/random`;
export const API_KEY_HEADER = 'x-api-key';
export const CF_CONNECTING_IP_HEADER = 'cf-connecting-ip';
export const VISITOR_ADDRESS_HEADER = 'x-real-ip';
export const MAX_ACTIVE_KEYS_PER_USER = 2;

export const DEV_WEB_PORT = 4321;
export const DEV_API_PORT = 8787;
export const DEV_POSTGRES_PORT = 5434;
export const DEV_INSPECTOR_PORT = 4322;

export const DEV_ES_PORT = 9201;
export const DEV_EDGE_PORT = 8090;
