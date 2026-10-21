#!/usr/bin/env bun

import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

import { err, ok, type Result } from 'neverthrow';

import { ROOT } from '../lib/root';

import { SECRETS_ENVIRONMENTS, type SecretsEnvironment } from './schema';
import { checkDecryptedValues, checkEncryptedNames } from './validate';

const AGE_KEY_PATHS = [
  join(homedir(), '.config/sops/age/keys.txt'),
  join(homedir(), 'Library/Application Support/sops/age/keys.txt'),
];

function isSecretsEnvironment(value: string): value is SecretsEnvironment {
  return SECRETS_ENVIRONMENTS.some((env) => env === value);
}

function hasAgeKey(): boolean {
  const keyFile = process.env['SOPS_AGE_KEY_FILE'];
  if (process.env['SOPS_AGE_KEY']) return true;
  return keyFile ? existsSync(keyFile) : AGE_KEY_PATHS.some((path) => existsSync(path));
}

async function decrypt(path: string): Promise<Result<string, string>> {
  if (!Bun.which('sops'))
    return err('sops is not installed (brew install sops), values were not checked');
  const proc = Bun.spawn(
    ['sops', 'decrypt', '--input-type', 'dotenv', '--output-type', 'dotenv', path],
    { stdout: 'pipe', stderr: 'pipe' }
  );
  const [stdout, stderr, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  return code === 0 ? ok(stdout) : err(`sops could not decrypt ${path}: ${stderr.trim()}`);
}

async function checkEnvironment(
  env: SecretsEnvironment,
  canDecrypt: boolean
): Promise<readonly string[] | undefined> {
  const path = join(ROOT, `secrets/${env}.enc.env`);
  if (!existsSync(path)) return undefined;
  const names = checkEncryptedNames(env, await Bun.file(path).text());
  if (names.length > 0 || !canDecrypt) return names;
  return (await decrypt(path)).match(
    (text) => checkDecryptedValues(env, text),
    (problem) => [problem]
  );
}

const requested = process.argv.slice(2);
const invalid = requested.filter((arg) => !isSecretsEnvironment(arg));
if (invalid.length > 0) {
  console.error(`usage: check.ts [${SECRETS_ENVIRONMENTS.join('|')}]...`);
  process.exit(1);
}
const environments =
  requested.length > 0
    ? requested.filter((arg) => isSecretsEnvironment(arg))
    : SECRETS_ENVIRONMENTS;

const canDecrypt = hasAgeKey();
let failed = false;
for (const env of environments) {
  const problems = await checkEnvironment(env, canDecrypt);
  if (problems === undefined || problems.length === 0) continue;
  failed = true;
  console.error(`secrets/${env}.enc.env:`);
  for (const problem of problems) console.error(`  ✗ ${problem}`);
}
process.exit(failed ? 1 : 0);
