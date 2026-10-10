import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const GUARD = join(import.meta.dir, 'forbidden-terms.sh');
const TERMS = '# local only\nzorblax\n\\bqux\\b\n';
const ZERO = '0'.repeat(40);

let root = '';
let work = '';

function isolatedEnv(): Record<string, string | undefined> {
  const inherited = Object.entries(process.env).filter(([name]) => !name.startsWith('GIT_'));
  return {
    ...Object.fromEntries(inherited),
    GIT_AUTHOR_NAME: 'test',
    GIT_AUTHOR_EMAIL: 'test@example.test',
    GIT_COMMITTER_NAME: 'test',
    GIT_COMMITTER_EMAIL: 'test@example.test',
  };
}

function gitIn(cwd: string, args: readonly string[]): string {
  const result = Bun.spawnSync(['git', ...args], { cwd, env: isolatedEnv() });
  if (result.exitCode !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr.toString()}`);
  return result.stdout.toString().trim();
}

function git(args: readonly string[]): string {
  return gitIn(work, args);
}

function guard(args: readonly string[], input = ''): number {
  const result = Bun.spawnSync([GUARD, ...args], {
    cwd: work,
    env: isolatedEnv(),
    stdin: new TextEncoder().encode(input),
  });
  return result.exitCode;
}

function setTerms(content: string | null): void {
  const path = join(work, '.git', 'info', 'forbidden-terms');
  if (content === null) rmSync(path, { force: true });
  else writeFileSync(path, content);
}

function stage(name: string, content: string): number {
  writeFileSync(join(work, name), content);
  git(['add', name]);
  const code = guard(['pre-commit']);
  git(['rm', '-q', '--cached', name]);
  rmSync(join(work, name));
  return code;
}

function message(text: string): number {
  const path = join(root, 'COMMIT_EDITMSG');
  writeFileSync(path, text);
  return guard(['commit-msg', path]);
}

function pushOf(sha: string): number {
  return guard(
    ['pre-push'],
    `refs/heads/main ${sha} refs/heads/main ${git(['rev-parse', 'origin/main'])}\n`
  );
}

beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), 'forbidden-terms-'));
  work = join(root, 'work');
  gitIn(root, ['init', '-q', '--bare', 'remote.git']);
  gitIn(root, ['init', '-q', '-b', 'main', 'work']);
  git(['remote', 'add', 'origin', join(root, 'remote.git')]);
  git(['commit', '--allow-empty', '-q', '-m', 'base']);
  git(['push', '-q', 'origin', 'main']);
  setTerms(TERMS);
});

afterAll(() => {
  rmSync(root, { recursive: true, force: true });
});

describe('the pre-commit check', () => {
  test('refuses a staged line that holds a term, in any case', () => {
    expect(stage('notes.txt', 'see ZorBlax here\n')).toBe(1);
  });

  test('matches a term written as a whole word only as a whole word', () => {
    expect(stage('notes.txt', 'quxx and aqux\n')).toBe(0);
    expect(stage('notes.txt', 'a qux b\n')).toBe(1);
  });

  test('refuses a staged file whose name holds a term', () => {
    expect(stage('zorblax.txt', 'clean\n')).toBe(1);
  });

  test('accepts clean staged changes', () => {
    expect(stage('notes.txt', 'clean\n')).toBe(0);
  });
});

describe('the commit-msg check', () => {
  test('refuses a message that holds a term', () => {
    expect(message('fix(data): import the zorblax poems\n')).toBe(1);
  });

  test('ignores the comment lines that git adds', () => {
    expect(message('fix(data): clean\n# zorblax\n')).toBe(0);
  });
});

describe('the pre-push check', () => {
  test('refuses a new commit whose message holds a term', () => {
    git(['commit', '--allow-empty', '-q', '-m', 'add qux data']);
    expect(pushOf(git(['rev-parse', 'HEAD']))).toBe(1);
    git(['reset', '-q', '--hard', 'origin/main']);
  });

  test('refuses a new commit that adds a line with a term', () => {
    writeFileSync(join(work, 'notes.txt'), 'from zorblax\n');
    git(['add', 'notes.txt']);
    git(['commit', '-q', '-m', 'add notes']);
    expect(pushOf(git(['rev-parse', 'HEAD']))).toBe(1);
    git(['reset', '-q', '--hard', 'origin/main']);
  });

  test('accepts clean new commits and a deleted branch', () => {
    git(['commit', '--allow-empty', '-q', '-m', 'clean work']);
    expect(pushOf(git(['rev-parse', 'HEAD']))).toBe(0);
    expect(
      guard(['pre-push'], `(delete) ${ZERO} refs/heads/old ${git(['rev-parse', 'HEAD'])}\n`)
    ).toBe(0);
    git(['reset', '-q', '--hard', 'origin/main']);
  });
});

describe('the text check', () => {
  test('refuses text that holds a term and accepts clean text', () => {
    expect(guard(['text'], 'an issue body about zorblax')).toBe(1);
    expect(guard(['text'], 'an issue body')).toBe(0);
  });
});

describe('a clone without a local list', () => {
  test('passes every check', () => {
    setTerms(null);
    try {
      expect(stage('zorblax.txt', 'zorblax\n')).toBe(0);
      expect(message('zorblax\n')).toBe(0);
      expect(guard(['text'], 'zorblax')).toBe(0);
    } finally {
      setTerms(TERMS);
    }
  });
});
