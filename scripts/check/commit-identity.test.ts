import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const GUARD = join(import.meta.dir, 'commit-identity.sh');
const ALLOWED = 'allowed@example.test';
const OTHER = 'other@example.test';

let root = '';
let work = '';

function git(args: readonly string[], email = ALLOWED): string {
  const result = Bun.spawnSync(['git', ...args], {
    cwd: work,
    env: {
      ...process.env,
      GIT_AUTHOR_NAME: 'test',
      GIT_AUTHOR_EMAIL: email,
      GIT_COMMITTER_NAME: 'test',
      GIT_COMMITTER_EMAIL: email,
    },
  });
  if (result.exitCode !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr.toString()}`);
  return result.stdout.toString().trim();
}

function commit(message: string, email = ALLOWED): string {
  git(['commit', '--allow-empty', '-q', '-m', message], email);
  return git(['rev-parse', 'HEAD']);
}

function prePush(localSha: string, remoteSha: string): number {
  const result = Bun.spawnSync([GUARD, 'pre-push'], {
    cwd: work,
    stdin: new TextEncoder().encode(
      `refs/heads/feature ${localSha} refs/heads/feature ${remoteSha}\n`
    ),
  });
  return result.exitCode;
}

beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), 'commit-identity-'));
  work = join(root, 'work');
  Bun.spawnSync(['git', 'init', '-q', '--bare', join(root, 'remote.git')]);
  Bun.spawnSync(['git', 'init', '-q', '-b', 'main', work]);
  git(['config', '--local', 'qafiyah.allowedEmail', ALLOWED]);
  git(['remote', 'add', 'origin', join(root, 'remote.git')]);
  commit('base');
  git(['push', '-q', 'origin', 'main']);
});

afterAll(() => {
  rmSync(root, { recursive: true, force: true });
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
