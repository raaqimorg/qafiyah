import { describe, expect, test } from 'bun:test';

import { checkDecryptedValues, checkEncryptedNames, dumpDirectories } from './validate';

const HEX_A = 'a'.repeat(64);
const HEX_B = 'b'.repeat(64);
const HEX_C = 'c'.repeat(64);
const DUMP_DIRS = ['0028_21_09_2026'];

const VALID_PROD = [
  'POSTGRES_PASSWORD=postgres-password-0001',
  'PG_READER_PASSWORD=reader-password-00002',
  'PG_ACCOUNTS_PASSWORD=accounts-password-003',
  'ELASTIC_PASSWORD=elastic-password-00004',
  'ES_READER_PASSWORD=es-reader-password-05',
  'PG_MONITOR_PASSWORD=pg-monitor-password-10',
  'ES_MONITOR_PASSWORD=es-monitor-password-11',
  'GRAFANA_ADMIN_PASSWORD=grafana-admin-password-12',
  'ENVIRONMENT=production',
  `API_KEY_INTERNAL=${HEX_A}`,
  `API_KEY_FULL=${HEX_B}`,
  'OAUTH_GOOGLE_CLIENT_ID=google-id',
  'OAUTH_GOOGLE_CLIENT_SECRET=google-secret',
  'OAUTH_GITHUB_CLIENT_ID=github-id',
  'OAUTH_GITHUB_CLIENT_SECRET=github-secret',
  `SESSION_STATE_SECRET=${HEX_C}`,
  'ACCOUNTS_BACKUP_BUCKET=qafiyah-accounts-backups',
  'ACCOUNTS_BACKUP_RECIPIENT=age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq',
  'ACCOUNTS_BACKUP_R2_ENDPOINT=https://0123456789abcdef.r2.cloudflarestorage.com',
  'ACCOUNTS_BACKUP_R2_ACCESS_KEY_ID=r2-access-key-id-0007',
  'ACCOUNTS_BACKUP_R2_SECRET_ACCESS_KEY=r2-secret-access-key-08',
  'CLOUDFLARE_ZONE_ID=0123456789abcdef0123456789abcdef',
  'CLOUDFLARE_CACHE_PURGE_TOKEN=cloudflare-purge-token-09',
  'TELEGRAM_BOT_TOKEN=123456:telegram-bot-token',
  'TELEGRAM_CHAT_ID=-1000000000',
].join('\n');

function withLine(base: string, key: string, value: string | undefined): string {
  const kept = base.split('\n').filter((line) => !line.startsWith(`${key}=`));
  return [...kept, ...(value === undefined ? [] : [`${key}=${value}`])].join('\n');
}

describe('checkDecryptedValues', () => {
  test('a complete prod file has no problems', () => {
    expect(checkDecryptedValues('prod', VALID_PROD, DUMP_DIRS)).toEqual([]);
  });

  test('a key set twice is reported with both lines', () => {
    const text = `${VALID_PROD}\nAPI_KEY_FULL=${HEX_B}`;
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'API_KEY_FULL is set 2 times (lines 11, 26)'
    );
  });

  test('a misspelled key is rejected as unknown', () => {
    const text = `${VALID_PROD}\nAPI_KEY_INTRENAL=x`;
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'API_KEY_INTRENAL is not a known key (add it to scripts/secrets/schema.ts)'
    );
  });

  test('a required prod key that is missing or empty is reported', () => {
    const text = withLine(VALID_PROD, 'SESSION_STATE_SECRET', '');
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'SESSION_STATE_SECRET is required in prod'
    );
  });

  test('dev does not require prod-only keys', () => {
    expect(checkDecryptedValues('dev', 'POSTGRES_PASSWORD=qafiyah', DUMP_DIRS)).toEqual([]);
  });

  test('a key forbidden in an environment is rejected there', () => {
    const text = `${VALID_PROD}\nANON_REQUESTS=60`;
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'ANON_REQUESTS must not be set in prod'
    );
  });

  test('the Telegram alert keys are required in prod', () => {
    const text = withLine(VALID_PROD, 'TELEGRAM_CHAT_ID', undefined);
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'TELEGRAM_CHAT_ID is required in prod'
    );
  });

  test('the Telegram alert keys are refused in dev, so a dev stack never sends alerts', () => {
    expect(
      checkDecryptedValues('dev', 'TELEGRAM_BOT_TOKEN=123456:telegram-bot-token', DUMP_DIRS)
    ).toContain('TELEGRAM_BOT_TOKEN must not be set in dev');
  });

  test('ENVIRONMENT must be exactly production in prod', () => {
    const text = withLine(VALID_PROD, 'ENVIRONMENT', 'Production');
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      "ENVIRONMENT must be exactly 'production'"
    );
  });

  test('a generated key that is not 64 hex characters is rejected', () => {
    const text = withLine(VALID_PROD, 'API_KEY_INTERNAL', 'short');
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'API_KEY_INTERNAL must be 64 lowercase hex characters (openssl rand -hex 32)'
    );
  });

  test('a short password is rejected in prod only', () => {
    const text = withLine(VALID_PROD, 'POSTGRES_PASSWORD', 'qafiyah');
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'POSTGRES_PASSWORD must be at least 16 characters in prod'
    );
    expect(checkDecryptedValues('dev', 'POSTGRES_PASSWORD=qafiyah', DUMP_DIRS)).toEqual([]);
  });

  test('an Elasticsearch password with a URL-reserved character is rejected', () => {
    for (const reserved of ['ab@cd', 'a/b', 'a#b', 'a?b', 'a:b', 'a b', 'a%b']) {
      const text = withLine(VALID_PROD, 'ES_READER_PASSWORD', `password-long-enough-${reserved}`);
      expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
        'ES_READER_PASSWORD must contain only URL-safe characters (letters, digits, and . _ ~ -)'
      );
    }
    const safe = withLine(VALID_PROD, 'ES_READER_PASSWORD', 'password-long-enough.url_safe~1');
    expect(checkDecryptedValues('prod', safe, DUMP_DIRS)).toEqual([]);
  });

  test('two secrets holding the same value are reported without the value', () => {
    const text = withLine(VALID_PROD, 'API_KEY_FULL', HEX_A);
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'API_KEY_INTERNAL and API_KEY_FULL hold the same value'
    );
  });

  test('half of a credential pair is reported', () => {
    const text = 'OAUTH_GITHUB_CLIENT_ID=github-id';
    expect(checkDecryptedValues('dev', text, DUMP_DIRS)).toContain(
      'OAUTH_GITHUB_CLIENT_ID and OAUTH_GITHUB_CLIENT_SECRET must be set together'
    );
  });

  test('the backup bucket cannot be the public assets bucket', () => {
    const text = withLine(VALID_PROD, 'ACCOUNTS_BACKUP_BUCKET', 'qafiyah-assets');
    expect(checkDecryptedValues('prod', text, DUMP_DIRS)).toContain(
      'ACCOUNTS_BACKUP_BUCKET must not be the public qafiyah-assets bucket'
    );
  });

  test('a dump key must name an existing dump directory', () => {
    expect(checkDecryptedValues('dev', 'DUMP_KEY__0028_21_09_2026=p', DUMP_DIRS)).toEqual([]);
    expect(checkDecryptedValues('dev', 'DUMP_KEY__0029_21_09_2026=p', DUMP_DIRS)).toContain(
      'DUMP_KEY__0029_21_09_2026 has no matching data/db/0029_21_09_2026 directory'
    );
  });
});

describe('checkEncryptedNames', () => {
  test('sops metadata lines are ignored and encrypted values pass', () => {
    const text = 'POSTGRES_PASSWORD=ENC[AES256_GCM,data:x]\nsops_version=3.13.3';
    expect(checkEncryptedNames('dev', text, DUMP_DIRS)).toEqual([]);
  });

  test('a value committed in plaintext is reported', () => {
    const text = 'POSTGRES_PASSWORD=hunter2\nsops_version=3.13.3';
    expect(checkEncryptedNames('dev', text, DUMP_DIRS)).toContain(
      'POSTGRES_PASSWORD (line 1) is stored unencrypted'
    );
  });

  test('missing required keys are caught without decrypting', () => {
    expect(checkEncryptedNames('prod', 'sops_version=3.13.3', DUMP_DIRS)).toContain(
      'API_KEY_INTERNAL is required in prod'
    );
  });
});

describe('dumpDirectories', () => {
  test('a dump directory counts when git tracks it but a sparse checkout left it off disk', () => {
    expect(
      dumpDirectories(['data/db/0031_23_09_2026/CHANGES.md.enc', 'data/db/README.md'], [])
    ).toEqual(['0031_23_09_2026']);
  });

  test('a dump directory counts when it is on disk but not committed yet', () => {
    expect(dumpDirectories([], ['0037_01_10_2026'])).toEqual(['0037_01_10_2026']);
  });

  test('the fallback sample is not a dump directory', () => {
    expect(
      dumpDirectories(['data/db/0000_default/manifest.json'], ['0000_default', '0031_23_09_2026'])
    ).toEqual(['0031_23_09_2026']);
  });
});
