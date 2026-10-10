import { describe, expect, test } from 'bun:test';

import { type CheckOutcome, type DoctorEnv, runChecks } from './doctor-checks';

const GIB = 1024 ** 3;

const HEALTHY: Readonly<Record<string, string>> = {
  'git --version': 'git version 2.50.1 (Apple Git-155)',
  'bash --version': 'GNU bash, version 5.3.9(1)-release (aarch64-apple-darwin25.1.0)',
  'docker --version': 'Docker version 29.4.0, build 9d7ad9f',
  'docker info --format {{.MemTotal}}': String(12 * GIB),
  'docker compose version --short': '5.1.2',
  'rustup --version': 'rustup 1.29.1 (d95a37b6a 2026-08-13)',
  'rustup toolchain list':
    'stable-aarch64-apple-darwin (default)\n1.99.0-aarch64-apple-darwin (active)',
  'shellcheck --version': 'ShellCheck - shell script analysis tool\nversion: 0.11.0',
  'actionlint --version': '1.7.12\ninstalled from Homebrew',
  'hadolint --version': 'Haskell Dockerfile Linter 2.15.1',
  'gh --version': 'gh version 2.88.1 (2026-03-12)',
  'gh auth status': 'github.com\n  ✓ Logged in to github.com',
};

type Fake = {
  readonly outputs?: Readonly<Record<string, string | null>>;
  readonly failing?: readonly string[];
  readonly env?: Partial<DoctorEnv>;
};

function fakeEnv({ outputs = {}, failing = [], env = {} }: Fake): DoctorEnv {
  const all = { ...HEALTHY, ...outputs };
  return {
    platform: 'darwin',
    bunVersion: '1.4.2',
    bunMinimum: '>=1',
    rustChannel: '1.99.0',
    hasNodeModules: true,
    which: (name) => name === 'sops' || name === 'age' || name === 'cargo',
    probe: async (command) => {
      const key = command.join(' ');
      const output = all[key];
      if (output === undefined || output === null) return undefined;
      return { code: failing.includes(key) ? 1 : 0, output };
    },
    ...env,
  };
}

async function outcome(name: string, fake: Fake = {}): Promise<CheckOutcome> {
  const outcomes = await runChecks(fakeEnv(fake), ['run', 'commit', 'optional']);
  const found = outcomes.find((item) => item.name === name);
  if (found === undefined) throw new Error(`no check named ${name}`);
  return found;
}

describe('runChecks', () => {
  test('passes every check on a ready machine', async () => {
    const outcomes = await runChecks(fakeEnv({}), ['run', 'commit', 'optional']);
    expect(outcomes.filter((item) => item.status !== 'ok')).toEqual([]);
  });

  test('runs only the groups it is asked for', async () => {
    const outcomes = await runChecks(fakeEnv({}), ['run']);
    expect(new Set(outcomes.map((item) => item.group))).toEqual(new Set(['run']));
  });

  test('fails the Bash that macOS ships, and says how to get a newer one', async () => {
    const bash = await outcome('Bash', {
      outputs: { 'bash --version': 'GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)' },
    });
    expect(bash.status).toBe('fail');
    expect(bash.detail).toBe('3.2.57 (needs 4)');
    expect(bash.fix).toContain('brew install bash');
  });

  test('fails a Docker Compose older than the !override tag', async () => {
    const compose = await outcome('Docker Compose', {
      outputs: { 'docker compose version --short': '2.24.3' },
    });
    expect(compose.status).toBe('fail');
    expect(compose.detail).toBe('2.24.3 (needs 2.24.4)');
  });

  test('reads a Docker Desktop Compose version with a suffix', async () => {
    const compose = await outcome('Docker Compose', {
      outputs: { 'docker compose version --short': '2.40.3-desktop.1' },
    });
    expect(compose.status).toBe('ok');
  });

  test('tells a stopped Docker apart from a missing one', async () => {
    const stopped = await outcome('Docker', { failing: ['docker info --format {{.MemTotal}}'] });
    expect(stopped.status).toBe('fail');
    expect(stopped.detail).toBe('installed, not running');
    const missing = await outcome('Docker', { outputs: { 'docker --version': null } });
    expect(missing.detail).toBe('not installed');
  });

  test('skips the memory check while Docker is stopped', async () => {
    const memory = await outcome('Docker memory', {
      failing: ['docker info --format {{.MemTotal}}'],
    });
    expect(memory.status).toBe('skip');
  });

  test('fails a Docker with too little memory for Elasticsearch', async () => {
    const memory = await outcome('Docker memory', {
      outputs: { 'docker info --format {{.MemTotal}}': String(2 * GIB) },
    });
    expect(memory.status).toBe('fail');
    expect(memory.detail).toBe('2.0 GB (needs 4 GB)');
  });

  test('accepts a Docker set to 4 GB, which reports a little less', async () => {
    const memory = await outcome('Docker memory', {
      outputs: { 'docker info --format {{.MemTotal}}': String(Math.round(3.8 * GIB)) },
    });
    expect(memory.status).toBe('ok');
  });

  test('fails cargo without rustup, because it ignores rust-toolchain.toml', async () => {
    const rustup = await outcome('rustup', { outputs: { 'rustup --version': null } });
    expect(rustup.status).toBe('fail');
    expect(rustup.detail).toContain('rust-toolchain.toml');
  });

  test('warns, not fails, when the pinned toolchain is not downloaded yet', async () => {
    const toolchain = await outcome('Rust toolchain', {
      outputs: { 'rustup toolchain list': 'stable-aarch64-apple-darwin (default)' },
    });
    expect(toolchain.status).toBe('warn');
    expect(toolchain.detail).toBe('1.99.0 not installed yet');
  });

  test('fails when bun install has not run', async () => {
    const install = await outcome('bun install', { env: { hasNodeModules: false } });
    expect(install.status).toBe('fail');
    expect(install.fix).toBe('bun install');
  });

  test('fails a Bun older than package.json allows', async () => {
    const bun = await outcome('Bun', { env: { bunVersion: '1.0.0', bunMinimum: '>=1.2' } });
    expect(bun.status).toBe('fail');
    expect(bun.detail).toBe('1.0.0 (needs 1.2)');
  });

  test('fails a missing pre-push linter in the commit group', async () => {
    const hadolint = await outcome('hadolint', { outputs: { 'hadolint --version': null } });
    expect(hadolint.group).toBe('commit');
    expect(hadolint.status).toBe('fail');
  });

  test('only warns about a GitHub CLI that is not logged in', async () => {
    const gh = await outcome('GitHub CLI', { failing: ['gh auth status'] });
    expect(gh.status).toBe('warn');
    expect(gh.fix).toBe('gh auth login');
  });

  test('gives Linux install commands on Linux', async () => {
    const bash = await outcome('Bash', {
      outputs: { 'bash --version': 'GNU bash, version 3.2.57(1)-release' },
      env: { platform: 'linux' },
    });
    expect(bash.fix).toContain('apt');
  });
});
