import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const GUARD = join(import.meta.dir, 'commit-identity.sh');
const ALLOWED = 'allowed@example.test';
const OTHER = 'other@example.test';

let root = '';
let work = '';

function isolatedEnv(email = ALLOWED): Record<string, string | undefined> {
  const inherited = Object.entries(process.env).filter(([name]) => !name.startsWith('GIT_'));
  return {
    ...Object.fromEntries(inherited),
    GIT_AUTHOR_NAME: 'test',
    GIT_AUTHOR_EMAIL: email,
    GIT_COMMITTER_NAME: 'test',
    GIT_COMMITTER_EMAIL: email,
  };
}

function gitIn(cwd: string, args: readonly string[], email = ALLOWED): string {
  const result = Bun.spawnSync(['git', ...args], { cwd, env: isolatedEnv(email) });
  if (result.exitCode !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr.toString()}`);
  return result.stdout.toString().trim();
}

function git(args: readonly string[], email = ALLOWED): string {
  return gitIn(work, args, email);
}

function commit(message: string, email = ALLOWED): string {
  git(['commit', '--allow-empty', '-q', '-m', message], email);
  return git(['rev-parse', 'HEAD']);
}

function prePush(localSha: string, remoteSha: string): number {
  const result = Bun.spawnSync([GUARD, 'pre-push'], {
    cwd: work,
    env: isolatedEnv(),
    stdin: new TextEncoder().encode(
      `refs/heads/feature ${localSha} refs/heads/feature ${remoteSha}\n`
    ),
  });
  return result.exitCode;
}

beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), 'commit-identity-'));
  work = join(root, 'work');
  gitIn(root, ['init', '-q', '--bare', 'remote.git']);
  gitIn(root, ['init', '-q', '-b', 'main', 'work']);
  git(['config', '--local', 'qafiyah.allowedEmail', ALLOWED]);
  git(['remote', 'add', 'origin', join(root, 'remote.git')]);
  commit('base');
  git(['push', '-q', 'origin', 'main']);
});

afterAll(() => {
  rmSync(root, { recursive: true, force: true });
});

describe('the scratch repositories', () => {
  test('ignore the GIT_DIR that a git hook in a worktree passes down', () => {
    const decoy = join(root, 'decoy');
    gitIn(root, ['init', '-q', 'decoy']);
    const inherited = process.env['GIT_DIR'];
    process.env['GIT_DIR'] = join(decoy, '.git');
    try {
      git(['config', '--local', 'qafiyah.probe', 'scratch']);
    } finally {
      if (inherited === undefined) delete process.env['GIT_DIR'];
      else process.env['GIT_DIR'] = inherited;
    }
    expect(readFileSync(join(decoy, '.git', 'config'), 'utf8')).not.toContain('probe');
    expect(git(['config', '--local', '--get', 'qafiyah.probe'])).toBe('scratch');
  });
});

describe('the pre-push commit identity guard', () => {
  test('refuses a new commit made under another email', () => {
    git(['switch', '-q', '-c', 'feature', 'main']);
    const pushed = commit('feature work');
    git(['push', '-q', 'origin', 'feature']);
    const foreign = commit('foreign work', OTHER);
    expect(prePush(foreign, pushed)).toBe(1);
    git(['reset', '-q', '--hard', pushed]);
  });

  test('accepts a branch that merged commits another identity already put on the remote', () => {
    git(['switch', '-q', 'main']);
    commit('merged on the server', OTHER);
    git(['push', '-q', 'origin', 'main']);
    git(['switch', '-q', 'feature']);
    const pushed = git(['rev-parse', 'HEAD']);
    git(['fetch', '-q', 'origin']);
    git(['merge', '-q', '--no-edit', 'origin/main']);
    expect(prePush(git(['rev-parse', 'HEAD']), pushed)).toBe(0);
  });

  test('still refuses a new foreign commit that rides along with merged remote commits', () => {
    const pushed = git(['rev-parse', 'origin/feature']);
    const foreign = commit('foreign work after the merge', OTHER);
    expect(prePush(foreign, pushed)).toBe(1);
  });
});
