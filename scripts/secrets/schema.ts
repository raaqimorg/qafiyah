export const SECRETS_ENVIRONMENTS = ['dev', 'prod'] as const;
export type SecretsEnvironment = (typeof SECRETS_ENVIRONMENTS)[number];

type Presence = 'required' | 'optional' | 'forbidden';

const SECRET_FORMATS = [
  'text',
  'password',
  'url-password',
  'hex64',
  'url',
  'positive-int',
  'production',
] as const;
export type SecretFormat = (typeof SECRET_FORMATS)[number];

export type KeySpec = {
  readonly presence: Readonly<Record<SecretsEnvironment, Presence>>;
  readonly format: SecretFormat;
  readonly isSecret: boolean;
};

const devOptional = { dev: 'optional', prod: 'optional' } as const;
const prodRequired = { dev: 'optional', prod: 'required' } as const;
const devOnly = { dev: 'optional', prod: 'forbidden' } as const;

export const DUMP_KEY_PATTERN = /^DUMP_KEY__(\d{4}_\d{2}_\d{2}_\d{4})$/;

export const DUMP_KEY_SPEC: KeySpec = { presence: devOptional, format: 'text', isSecret: true };

export const KEY_SPECS: Readonly<Record<string, KeySpec>> = {
  POSTGRES_USER: { presence: devOptional, format: 'text', isSecret: false },
  POSTGRES_DB: { presence: devOptional, format: 'text', isSecret: false },
  POSTGRES_PASSWORD: { presence: prodRequired, format: 'password', isSecret: true },
  PG_READER_PASSWORD: { presence: prodRequired, format: 'password', isSecret: true },
  PG_ACCOUNTS_PASSWORD: { presence: prodRequired, format: 'password', isSecret: true },
  ELASTIC_PASSWORD: { presence: prodRequired, format: 'url-password', isSecret: true },
  ES_READER_PASSWORD: { presence: prodRequired, format: 'url-password', isSecret: true },
  PG_MONITOR_PASSWORD: { presence: prodRequired, format: 'password', isSecret: true },
  ES_MONITOR_PASSWORD: { presence: prodRequired, format: 'password', isSecret: true },
  ENVIRONMENT: {
    presence: { dev: 'forbidden', prod: 'required' },
    format: 'production',
    isSecret: false,
  },
  API_KEY_INTERNAL: { presence: prodRequired, format: 'hex64', isSecret: true },
  API_KEY_FULL: { presence: prodRequired, format: 'hex64', isSecret: true },
  ANON_REQUESTS: { presence: devOnly, format: 'positive-int', isSecret: false },
  SENTRY_DSN_API: { presence: devOptional, format: 'url', isSecret: false },
  SENTRY_AUTH_TOKEN: { presence: devOptional, format: 'text', isSecret: true },
  OAUTH_GOOGLE_CLIENT_ID: { presence: prodRequired, format: 'text', isSecret: false },
  OAUTH_GOOGLE_CLIENT_SECRET: { presence: prodRequired, format: 'text', isSecret: true },
  OAUTH_GITHUB_CLIENT_ID: { presence: prodRequired, format: 'text', isSecret: false },
  OAUTH_GITHUB_CLIENT_SECRET: { presence: prodRequired, format: 'text', isSecret: true },
  SESSION_STATE_SECRET: { presence: prodRequired, format: 'hex64', isSecret: true },
  ACCOUNTS_BACKUP_BUCKET: { presence: prodRequired, format: 'text', isSecret: false },
  ACCOUNTS_BACKUP_RECIPIENT: { presence: prodRequired, format: 'text', isSecret: false },
  ACCOUNTS_BACKUP_R2_ENDPOINT: { presence: prodRequired, format: 'url', isSecret: false },
  ACCOUNTS_BACKUP_R2_ACCESS_KEY_ID: { presence: prodRequired, format: 'text', isSecret: true },
  ACCOUNTS_BACKUP_R2_SECRET_ACCESS_KEY: {
    presence: prodRequired,
    format: 'text',
    isSecret: true,
  },
  CLOUDFLARE_ZONE_ID: { presence: prodRequired, format: 'text', isSecret: false },
  CLOUDFLARE_CACHE_PURGE_TOKEN: { presence: prodRequired, format: 'text', isSecret: true },
  CLOUDFLARE_API_TOKEN: { presence: devOptional, format: 'text', isSecret: true },
};

export const PAIRED_KEYS: readonly (readonly [string, string])[] = [
  ['OAUTH_GOOGLE_CLIENT_ID', 'OAUTH_GOOGLE_CLIENT_SECRET'],
  ['OAUTH_GITHUB_CLIENT_ID', 'OAUTH_GITHUB_CLIENT_SECRET'],
  ['ACCOUNTS_BACKUP_BUCKET', 'ACCOUNTS_BACKUP_RECIPIENT'],
  ['ACCOUNTS_BACKUP_R2_ACCESS_KEY_ID', 'ACCOUNTS_BACKUP_R2_SECRET_ACCESS_KEY'],
];

export const PUBLIC_ASSETS_BUCKET = 'qafiyah-assets';

export const PROD_PASSWORD_MIN_LENGTH = 16;
