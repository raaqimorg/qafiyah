import { describe, expect, test } from 'bun:test';

import { serviceUrls } from './service-urls';

describe('serviceUrls', () => {
  test('derives the three connection strings from the dev defaults and ports', () => {
    const urls = serviceUrls({ env: {}, offset: 0, orbstack: false, suffix: '' });
    expect(urls.database).toBe(
      'postgresql://qafiyah_api:qafiyah-dev-pg-reader@localhost:5434/qafiyah'
    );
    expect(urls.accounts).toBe(
      'postgresql://qafiyah_accounts:qafiyah-dev-pg-accounts@localhost:5434/qafiyah_accounts'
    );
    expect(urls.elasticsearch).toBe('http://qafiyah_api:qafiyah-dev-reader@localhost:9201');
    expect(urls.elasticsearchAdmin).toBe('http://elastic:qafiyah-dev-es@localhost:9201');
  });

  test('honors explicit passwords, a worktree offset, and the OrbStack container host', () => {
    const urls = serviceUrls({
      env: {
        PG_READER_PASSWORD: 'r',
        PG_ACCOUNTS_PASSWORD: 'a',
        ES_READER_PASSWORD: 'e',
        ELASTIC_PASSWORD: 'x',
        POSTGRES_DB: 'db',
      },
      offset: 10,
      orbstack: true,
      suffix: '-wt',
    });
    expect(urls.database).toBe('postgresql://qafiyah_api:r@localhost:5444/db');
    expect(urls.accounts).toBe('postgresql://qafiyah_accounts:a@localhost:5444/qafiyah_accounts');
    expect(urls.elasticsearch).toBe('http://qafiyah_api:e@qafiyah-dev-wt-es.orb.local:9200');
    expect(urls.elasticsearchAdmin).toBe('http://elastic:x@qafiyah-dev-wt-es.orb.local:9200');
  });
});
