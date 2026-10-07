import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const WEB = fileURLToPath(new URL('../../../', import.meta.url));

function dsnHost(file: string): string {
  const match = /dsn: 'https:\/\/[^@]+@([^/]+)\//.exec(readFileSync(`${WEB}${file}`, 'utf8'));
  if (!match?.[1]) throw new Error(`no DSN in ${file}`);
  return match[1];
}

function connectSources(file: string): string[] {
  const match = /connect-src ([^;]+);/.exec(readFileSync(`${WEB}${file}`, 'utf8'));
  if (!match?.[1]) throw new Error(`no connect-src in ${file}`);
  return match[1].split(' ');
}

describe('browser error reports', () => {
  it('go to the same Sentry ingest host as the server reports', () => {
    expect(dsnHost('sentry.client.config.js')).toBe(dsnHost('sentry.server.config.js'));
  });

  it.each(['nginx-csp.conf', 'nginx-csp-api.conf'])(
    'are allowed by the connect-src of %s',
    (file) => {
      expect(connectSources(file)).toContain(`https://${dsnHost('sentry.client.config.js')}`);
    }
  );
});
