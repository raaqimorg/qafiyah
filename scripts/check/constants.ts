#!/usr/bin/env bun

import { readFileSync } from 'node:fs';
import { join, relative } from 'node:path';

import {
  API_KEY_HEADER,
  API_RANDOM_POEM_PATH,
  API_V1_PREFIX,
  CF_CONNECTING_IP_HEADER,
  DEV_API_PORT,
  DEV_POSTGRES_PORT,
  DEV_WEB_PORT,
  ES_MAX_RESULT_WINDOW,
  GITHUB_AVATARS_URL,
  GITHUB_DB_DUMPS_URL,
  GITHUB_URL,
  LIST_POETS_MAX_PAGE,
  MAX_ACTIVE_KEYS_PER_USER,
  MAX_FILTER_SLUGS,
  MAX_QUERY_LENGTH,
  MAX_TWEET_LENGTH,
  POEMS_PER_PAGE,
  POETS_LIST_MAX_RESULT_WINDOW,
  PROD_API_URL,
  PROD_DOMAIN,
  PROD_SITE_URL,
  RAAQIM_URL,
  SECONDS_PER_HOUR,
  SECURITY_EMAIL,
  SEARCH_POEMS_MAX_PAGE,
  SEARCH_POEMS_PER_PAGE,
  SEARCH_TYPE_VALUES,
  SEARCH_POETS_MAX_PAGE,
  SEARCH_POETS_PER_PAGE,
  SITEMAP_POEMS_PER_SHARD,
  SITEMAP_POETS_PER_SHARD,
  TELEGRAM_URL,
  VISITOR_ADDRESS_HEADER,
  X_INTENT_TWEET_URL,
  X_PROFILE_URL,
} from '@qafiyah/config';

import { ROOT } from '../lib/root';
import { walkFiles } from '../lib/walk';

const SCAN_DIRS = ['apps/web/src', 'apps/inspector/src', 'scripts'];

const TEST_FILE_RE = /\.(test|spec)\.[tj]sx?$/;

const isGenerated = (path: string): boolean => path.endsWith('.gen.ts');

const isTestFile = (path: string): boolean =>
  TEST_FILE_RE.test(path) || path.endsWith('test-schemas.ts') || path.endsWith('test-utils.ts');

const isRulesFile = (path: string): boolean => path.endsWith('check/constants.ts');

const RULES: readonly { re: RegExp; name: string; constant: string }[] = [
  {
    re: /qafiyah\.com/g,
    name: 'production domain',
    constant: 'PROD_DOMAIN / PROD_SITE_URL / PROD_API_URL',
  },
  { re: /localhost:4321\b/g, name: 'dev web URL', constant: 'DEV_WEB_PORT' },
  { re: /localhost:8787\b/g, name: 'dev API URL', constant: 'DEV_API_PORT' },
  { re: /\bport\s*[:=]\s*4321\b/g, name: 'dev web port', constant: 'DEV_WEB_PORT' },
  { re: /\bport\s*[:=]\s*8787\b/g, name: 'dev API port', constant: 'DEV_API_PORT' },
  { re: /localhost:5434\b/g, name: 'dev Postgres URL', constant: 'DEV_POSTGRES_PORT' },
  { re: /\bport\s*[:=]\s*5434\b/g, name: 'dev Postgres port', constant: 'DEV_POSTGRES_PORT' },
];

type Violation = { file: string; line: number; match: string; name: string; constant: string };

const violations: Violation[] = [];

for (const dir of SCAN_DIRS) {
  const abs = join(ROOT, dir);
  for (const file of walkFiles(abs)) {
    if (isTestFile(file) || isGenerated(file) || isRulesFile(file)) continue;
    const content = readFileSync(file, 'utf8');
    for (const rule of RULES) {
      for (const match of content.matchAll(rule.re)) {
        const line = content.slice(0, match.index ?? 0).split('\n').length;
        violations.push({
          file: relative(ROOT, file),
          line,
          match: match[0],
          name: rule.name,
          constant: rule.constant,
        });
      }
    }
  }
}

const PINNED: readonly { constant: string; value: string; files: readonly string[] }[] = [
  {
    constant: 'DEV_WEB_PORT',
    value: String(DEV_WEB_PORT),
    files: ['apps/web/nginx.conf', 'apps/web/astro.config.mjs'],
  },
  {
    constant: 'DEV_API_PORT',
    value: String(DEV_API_PORT),
    files: ['apps/web/nginx.conf', 'docker-compose.yml'],
  },
  {
    constant: 'DEV_POSTGRES_PORT',
    value: String(DEV_POSTGRES_PORT),
    files: ['docker-compose.dev.yml'],
  },
  {
    constant: 'API_RANDOM_POEM_PATH',
    value: `route("${API_RANDOM_POEM_PATH.slice(API_V1_PREFIX.length)}"`,
    files: ['apps/api/src/routes/poems.rs'],
  },
  {
    constant: 'API_RANDOM_POEM_PATH',
    value: `'${API_RANDOM_POEM_PATH.slice(API_V1_PREFIX.length + 1)}'`,
    files: ['apps/web/src/lib/api/proxy-allowlist.ts'],
  },
  {
    constant: 'POETS_LIST_MAX_RESULT_WINDOW',
    value: `"max_result_window": ${POETS_LIST_MAX_RESULT_WINDOW}`,
    files: ['crates/elasticsearch/schema.json'],
  },
  {
    constant: 'PROD_DOMAIN',
    value: PROD_DOMAIN,
    files: ['docker-compose.yml', 'scripts/deploy/vps.sh'],
  },
  {
    constant: 'PROD_SITE_URL',
    value: PROD_SITE_URL,
    files: ['apps/web/astro.config.mjs'],
  },
  {
    constant: 'VISITOR_ADDRESS_HEADER',
    value: `proxy_set_header ${VISITOR_ADDRESS_HEADER} $remote_addr;`,
    files: ['apps/web/nginx.conf'],
  },
];

type Drift = { file: string; constant: string; value: string };

const drift: Drift[] = [];

for (const pin of PINNED) {
  for (const file of pin.files) {
    const content = readFileSync(join(ROOT, file), 'utf8');
    if (!content.includes(pin.value)) {
      drift.push({ file, constant: pin.constant, value: pin.value });
    }
  }
}

type RustValue = number | string | readonly string[];

const RUST_CONSTANTS: readonly {
  readonly file: string;
  readonly values: Readonly<Record<string, RustValue>>;
}[] = [
  {
    file: 'apps/api/src/constants.rs',
    values: {
      POEMS_PER_PAGE,
      SITEMAP_POEMS_PER_SHARD,
      SITEMAP_POETS_PER_SHARD,
      MAX_FILTER_SLUGS,
      MAX_QUERY_LENGTH,
      SEARCH_POEMS_PER_PAGE,
      SEARCH_POETS_PER_PAGE,
      SEARCH_TYPE_VALUES,
      ES_MAX_RESULT_WINDOW,
      SEARCH_POEMS_MAX_PAGE,
      SEARCH_POETS_MAX_PAGE,
      POETS_LIST_MAX_RESULT_WINDOW,
      LIST_POETS_MAX_PAGE,
      MAX_TWEET_LENGTH,
      API_V1_PREFIX,
      API_KEY_HEADER,
      CF_CONNECTING_IP_HEADER,
      MAX_ACTIVE_KEYS_PER_USER,
      PROD_DOMAIN,
      PROD_SITE_URL,
      PROD_API_URL,
      X_PROFILE_URL,
      TELEGRAM_URL,
      GITHUB_URL,
      GITHUB_DB_DUMPS_URL,
      GITHUB_AVATARS_URL,
      RAAQIM_URL,
      X_INTENT_TWEET_URL,
      SECURITY_EMAIL,
      SECONDS_PER_HOUR,
    },
  },
];

const RUST_LITERAL_RE = (name: string) =>
  new RegExp(`pub const ${name}\\s*:\\s*[^=]+=\\s*([^;]+);`);

const readRustLiteral = (literal: string, expected: RustValue): RustValue | undefined => {
  if (Array.isArray(expected)) {
    try {
      const parsed: unknown = JSON.parse(literal);
      return Array.isArray(parsed) ? (parsed as readonly string[]) : undefined;
    } catch {
      return undefined;
    }
  }
  return typeof expected === 'string'
    ? literal.replace(/^"(.*)"$/s, '$1')
    : Number(literal.replaceAll('_', ''));
};

const show = (value: RustValue | undefined): string =>
  Array.isArray(value) ? JSON.stringify(value) : String(value);

const sameValue = (actual: RustValue | undefined, expected: RustValue): boolean =>
  Array.isArray(expected)
    ? JSON.stringify(actual) === JSON.stringify(expected)
    : actual === expected;

for (const rust of RUST_CONSTANTS) {
  const content = readFileSync(join(ROOT, rust.file), 'utf8');
  for (const [name, expected] of Object.entries(rust.values)) {
    const declaration = RUST_LITERAL_RE(name).exec(content);
    const literal = declaration?.[1]?.trim();
    if (literal === undefined) {
      drift.push({
        file: rust.file,
        constant: name,
        value: `${show(expected)} (no declaration found)`,
      });
      continue;
    }
    const actual = readRustLiteral(literal, expected);
    if (!sameValue(actual, expected)) {
      drift.push({
        file: rust.file,
        constant: name,
        value: `${show(expected)}, found ${show(actual)}`,
      });
    }
  }
}

if (drift.length > 0) {
  console.error('Infra drifted from @qafiyah/config:\n');
  for (const d of drift) {
    console.error(`  ${d.file} no longer agrees with ${d.constant} (expected ${d.value})`);
  }
  console.error(
    [
      '',
      'Rule: infra files cannot import @qafiyah/config, so they are pinned to it.',
      '      Update the file to match the constant, or update PINNED in this script.',
      '',
    ].join('\n')
  );
  process.exit(1);
}

if (violations.length > 0) {
  console.error('Hardcoded constants detected:\n');
  for (const violation of violations) {
    console.error(
      `  ${violation.file}:${violation.line}  "${violation.match}", use ${violation.constant} from @qafiyah/config`
    );
  }
  console.error(
    [
      '',
      'Rule: brand strings, URLs, and dev ports live in @qafiyah/config (config.ts).',
      '      Import from @qafiyah/config instead of hardcoding.',
      '',
    ].join('\n')
  );
  process.exit(1);
}

console.log(
  `No hardcoded centralized constants in app source; ${PINNED.length} infra pins and ${RUST_CONSTANTS.length} rust constant files agree with @qafiyah/config.`
);
