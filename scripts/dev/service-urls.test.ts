import { describe, expect, test } from 'bun:test';

import { serviceUrls } from './service-urls';

describe('serviceUrls', () => {
  test('derives the three connection strings from the dev defaults and ports', () => {
    const urls = serviceUrls({ env: {}, offset: 0 });
    expect(urls.database).toBe(
      'postgresql://qafiyah_api:qafiyah-dev-pg-reader@localhost:5434/qafiyah'
    );
    expect(urls.accounts).toBe(
      'postgresql://qafiyah_accounts:qafiyah-dev-pg-accounts@localhost:5434/qafiyah_accounts'
    );
    expect(urls.elasticsearch).toBe('http://qafiyah_api:qafiyah-dev-reader@localhost:9201');
    expect(urls.elasticsearchAdmin).toBe('http://elastic:qafiyah-dev-es@localhost:9201');
  });

  test('honors explicit passwords and shifts every published port by the worktree offset', () => {
    const urls = serviceUrls({
      env: {
        PG_READER_PASSWORD: 'r',
        PG_ACCOUNTS_PASSWORD: 'a',
        ES_READER_PASSWORD: 'e',
        ELASTIC_PASSWORD: 'x',
        POSTGRES_DB: 'db',
      },
      offset: 10,
    });
    expect(urls.database).toBe('postgresql://qafiyah_api:r@localhost:5444/db');
    expect(urls.accounts).toBe('postgresql://qafiyah_accounts:a@localhost:5444/qafiyah_accounts');
    expect(urls.elasticsearch).toBe('http://qafiyah_api:e@localhost:9211');
    expect(urls.elasticsearchAdmin).toBe('http://elastic:x@localhost:9211');
  });
});
