#!/usr/bin/env bun

import { err, ok, type Result } from 'neverthrow';

import { ROOT } from '../lib/root';
import { dumpDirectories } from '../secrets/validate';

export const TAXONOMY_OPTIONS_PATH = 'apps/web/src/lib/generated/taxonomy/taxonomy-options.gen.ts';

export type History = {
  readonly lastCommitTouching: (path: string) => string | undefined;
  readonly isAncestor: (ancestor: string, descendant: string) => boolean;
};

export function checkTaxonomyOptionsFollowNewestDump(
  trackedPaths: readonly string[],
  history: History
): Result<string, string> {
  const newest = [...dumpDirectories(trackedPaths, [])].sort().at(-1);
  if (newest === undefined) return ok('No committed dump, nothing to compare.');
  const dumpCommit = history.lastCommitTouching(`data/db/${newest}`);
  if (dumpCommit === undefined) return ok(`Dump ${newest} has no commit yet.`);
  const optionsCommit = history.lastCommitTouching(TAXONOMY_OPTIONS_PATH);
  if (optionsCommit !== undefined && history.isAncestor(dumpCommit, optionsCommit)) {
    return ok(`Taxonomy options were regenerated with or after dump ${newest}.`);
  }
  return err(
    `Dump ${newest} was committed after the last regeneration of ${TAXONOMY_OPTIONS_PATH}, ` +
      'so the site would show the filter counts of an older dump. Regenerate it from a local stack ' +
      'running that dump (bun run dev, then bun run taxonomy:generate:dev) and commit it.'
  );
}

function git(args: readonly string[]): { readonly exitCode: number; readonly stdout: string } {
  const run = Bun.spawnSync(['git', ...args], { cwd: ROOT });
  return { exitCode: run.exitCode, stdout: run.stdout.toString().trim() };
}

if (import.meta.main) {
  if (git(['rev-parse', '--is-shallow-repository']).stdout === 'true') {
    console.log('Skipped: a shallow clone has no history to compare (the pre-push gate runs it).');
    process.exit(0);
  }
  const tracked = git(['ls-files', '-z', '--', 'data/db']);
  if (tracked.exitCode !== 0) {
    console.error('git ls-files failed');
    process.exit(1);
  }
  const history: History = {
    lastCommitTouching: (path) => git(['log', '-1', '--format=%H', '--', path]).stdout || undefined,
    isAncestor: (ancestor, descendant) =>
      git(['merge-base', '--is-ancestor', ancestor, descendant]).exitCode === 0,
  };
  checkTaxonomyOptionsFollowNewestDump(tracked.stdout.split('\0'), history).match(
    (message) => console.log(message),
    (message) => {
      console.error(message);
      process.exit(1);
    }
  );
}
