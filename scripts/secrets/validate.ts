import { type DotenvEntry, parseDotenv } from './dotenv';
import {
  DUMP_KEY_PATTERN,
  DUMP_KEY_SPEC,
  KEY_SPECS,
  type KeySpec,
  PAIRED_KEYS,
  PROD_PASSWORD_MIN_LENGTH,
  PUBLIC_ASSETS_BUCKET,
  type SecretFormat,
  type SecretsEnvironment,
} from './schema';

const SOPS_METADATA_PREFIX = 'sops_';
const ENCRYPTED_VALUE_PREFIX = 'ENC[';
const DUMPS_PATH_PREFIX = 'data/db/';
const DEFAULT_DUMP_DIR = '0000_default';

type FormatCheck = (value: string, env: SecretsEnvironment) => string | undefined;

const URL_SAFE_PASSWORD = /^[A-Za-z0-9._~-]+$/;

const passwordTooShort = (value: string, env: SecretsEnvironment) =>
  env === 'prod' && value.length < PROD_PASSWORD_MIN_LENGTH
    ? `must be at least ${PROD_PASSWORD_MIN_LENGTH} characters in prod`
    : undefined;

const FORMAT_CHECKS: Readonly<Record<SecretFormat, FormatCheck>> = {
  text: () => undefined,
  password: passwordTooShort,
  'url-password': (value, env) =>
    passwordTooShort(value, env) ??
    (URL_SAFE_PASSWORD.test(value)
      ? undefined
      : 'must contain only URL-safe characters (letters, digits, and . _ ~ -)'),
  hex64: (value) =>
    /^[0-9a-f]{64}$/.test(value)
      ? undefined
      : 'must be 64 lowercase hex characters (openssl rand -hex 32)',
  url: (value) => (URL.canParse(value) ? undefined : 'must be a valid URL'),
  'positive-int': (value) => (/^[1-9]\d*$/.test(value) ? undefined : 'must be a positive integer'),
  production: (value) => (value === 'production' ? undefined : "must be exactly 'production'"),
};

function specFor(key: string, dumpDirs: readonly string[]): KeySpec | string {
  const dump = DUMP_KEY_PATTERN.exec(key);
  if (dump) {
    return dumpDirs.includes(dump[1] ?? '')
      ? DUMP_KEY_SPEC
      : `${key} has no matching data/db/${dump[1]} directory`;
  }
  return KEY_SPECS[key] ?? `${key} is not a known key (add it to scripts/secrets/schema.ts)`;
}

function structuralProblems(
  env: SecretsEnvironment,
  entries: readonly DotenvEntry[],
  malformedLines: readonly number[],
  dumpDirs: readonly string[],
  isSet: (entry: DotenvEntry) => boolean
): string[] {
  const problems = malformedLines.map((line) => `line ${line} is not KEY=value`);

  const linesByKey = new Map<string, number[]>();
  for (const { key, line } of entries) linesByKey.set(key, [...(linesByKey.get(key) ?? []), line]);
  for (const [key, lines] of linesByKey) {
    if (lines.length > 1)
      problems.push(`${key} is set ${lines.length} times (lines ${lines.join(', ')})`);
    const spec = specFor(key, dumpDirs);
    if (typeof spec === 'string') problems.push(spec);
    else if (spec.presence[env] === 'forbidden') problems.push(`${key} must not be set in ${env}`);
  }

  const setKeys = new Set(entries.filter((entry) => isSet(entry)).map(({ key }) => key));
  for (const [key, spec] of Object.entries(KEY_SPECS)) {
    if (spec.presence[env] === 'required' && !setKeys.has(key)) {
      problems.push(`${key} is required in ${env}`);
    }
  }
  return problems;
}

export function checkEncryptedNames(
  env: SecretsEnvironment,
  text: string,
  dumpDirs: readonly string[]
): readonly string[] {
  const parsed = parseDotenv(text);
  const entries = parsed.entries.filter(({ key }) => !key.startsWith(SOPS_METADATA_PREFIX));
  const plaintext = entries
    .filter(({ value }) => !value.startsWith(ENCRYPTED_VALUE_PREFIX))
    .map(({ key, line }) => `${key} (line ${line}) is stored unencrypted`);
  return [
    ...plaintext,
    ...structuralProblems(env, entries, parsed.malformedLines, dumpDirs, () => true),
  ];
}

export function checkDecryptedValues(
  env: SecretsEnvironment,
  text: string,
  dumpDirs: readonly string[]
): readonly string[] {
  const { entries, malformedLines } = parseDotenv(text);
  const problems = structuralProblems(
    env,
    entries,
    malformedLines,
    dumpDirs,
    ({ value }) => value !== ''
  );
  const values = new Map(entries.filter(({ value }) => value !== '').map((e) => [e.key, e.value]));

  for (const [key, value] of values) {
    const spec = specFor(key, dumpDirs);
    if (typeof spec === 'string') continue;
    const formatProblem = FORMAT_CHECKS[spec.format](value, env);
    if (formatProblem) problems.push(`${key} ${formatProblem}`);
  }

  for (const [first, second] of PAIRED_KEYS) {
    if (values.has(first) !== values.has(second)) {
      problems.push(`${first} and ${second} must be set together`);
    }
  }

  if (values.get('ACCOUNTS_BACKUP_BUCKET') === PUBLIC_ASSETS_BUCKET) {
    problems.push(`ACCOUNTS_BACKUP_BUCKET must not be the public ${PUBLIC_ASSETS_BUCKET} bucket`);
  }

  const secretKeysByValue = new Map<string, string[]>();
  for (const [key, value] of values) {
    const spec = specFor(key, dumpDirs);
    if (typeof spec === 'string' || !spec.isSecret) continue;
    secretKeysByValue.set(value, [...(secretKeysByValue.get(value) ?? []), key]);
  }
  for (const keys of secretKeysByValue.values()) {
    if (keys.length > 1) problems.push(`${keys.join(' and ')} hold the same value`);
  }

  return problems;
}

export function dumpDirectories(
  trackedPaths: readonly string[],
  directoriesOnDisk: readonly string[]
): readonly string[] {
  const tracked = trackedPaths
    .filter((path) => path.startsWith(DUMPS_PATH_PREFIX))
    .map((path) => path.slice(DUMPS_PATH_PREFIX.length).split('/'))
    .filter((segments) => segments.length > 1)
    .map(([directory]) => directory ?? '');
  return [...new Set([...tracked, ...directoriesOnDisk])].filter(
    (directory) => directory !== '' && directory !== DEFAULT_DUMP_DIR
  );
}
