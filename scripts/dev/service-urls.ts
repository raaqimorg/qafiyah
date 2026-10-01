import { DEV_ES_PORT, DEV_POSTGRES_PORT } from '@qafiyah/config';

const DEV_PASSWORD_DEFAULTS = {
  PG_READER_PASSWORD: 'qafiyah-dev-pg-reader',
  PG_ACCOUNTS_PASSWORD: 'qafiyah-dev-pg-accounts',
  ES_READER_PASSWORD: 'qafiyah-dev-reader',
  ELASTIC_PASSWORD: 'qafiyah-dev-es',
} as const;

export function serviceUrls({
  env,
  offset,
  orbstack,
  suffix,
}: {
  readonly env: Readonly<Record<string, string | undefined>>;
  readonly offset: number;
  readonly orbstack: boolean;
  readonly suffix: string;
}): {
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
  const pgPort = DEV_POSTGRES_PORT + offset;
  const esHost = orbstack ? `qafiyah-dev${suffix}-es.orb.local` : 'localhost';
  const esPort = orbstack ? 9200 : DEV_ES_PORT + offset;
  return {
    database: `postgresql://qafiyah_api:${reader}@localhost:${pgPort}/${database}`,
    accounts: `postgresql://qafiyah_accounts:${accounts}@localhost:${pgPort}/qafiyah_accounts`,
    elasticsearch: `http://qafiyah_api:${esReader}@${esHost}:${esPort}`,
    elasticsearchAdmin: `http://elastic:${esAdmin}@${esHost}:${esPort}`,
  };
}
