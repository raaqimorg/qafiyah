import { DEV_ES_PORT, DEV_POSTGRES_PORT } from '@qafiyah/config';

const DEV_PASSWORD_DEFAULTS = {
  PG_READER_PASSWORD: 'qafiyah-dev-pg-reader',
  PG_ACCOUNTS_PASSWORD: 'qafiyah-dev-pg-accounts',
  ES_READER_PASSWORD: 'qafiyah-dev-reader',
  ELASTIC_PASSWORD: 'qafiyah-dev-es',
} as const;

export function serviceUrls(env: Readonly<Record<string, string | undefined>>): {
  readonly database: string;
  readonly accounts: string;
  readonly elasticsearch: string;
  readonly elasticsearchAdmin: string;
} {
  const reader = env['PG_READER_PASSWORD'] ?? DEV_PASSWORD_DEFAULTS.PG_READER_PASSWORD;
  const accounts = env['PG_ACCOUNTS_PASSWORD'] ?? DEV_PASSWORD_DEFAULTS.PG_ACCOUNTS_PASSWORD;
  const esReader = env['ES_READER_PASSWORD'] ?? DEV_PASSWORD_DEFAULTS.ES_READER_PASSWORD;
  const esAdmin = env['ELASTIC_PASSWORD'] ?? DEV_PASSWORD_DEFAULTS.ELASTIC_PASSWORD;
  const database = env['POSTGRES_DB'] ?? 'qafiyah';
  return {
    database: `postgresql://qafiyah_api:${reader}@localhost:${DEV_POSTGRES_PORT}/${database}`,
    accounts: `postgresql://qafiyah_accounts:${accounts}@localhost:${DEV_POSTGRES_PORT}/qafiyah_accounts`,
    elasticsearch: `http://qafiyah_api:${esReader}@localhost:${DEV_ES_PORT}`,
    elasticsearchAdmin: `http://elastic:${esAdmin}@localhost:${DEV_ES_PORT}`,
  };
}
