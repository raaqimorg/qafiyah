import { existsSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const SOURCE_EXTS = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.astro'] as const;

export const IGNORED_DIRS: ReadonlySet<string> = new Set([
  '.astro',
  '.git',
  '.husky',
  '.next',
  '.turbo',
  '.worktrees',
  '.wrangler',
  'build',
  'coverage',
  'dist',
  'node_modules',
  'target',
]);

type WalkOptions = {
  readonly exts?: readonly string[];
  readonly ignoredDirs?: ReadonlySet<string>;
};

export function* walkFiles(dir: string, options: WalkOptions = {}): Generator<string> {
  if (!existsSync(dir)) return;
  const exts = options.exts ?? SOURCE_EXTS;
  const ignored = options.ignoredDirs ?? IGNORED_DIRS;
  for (const entry of readdirSync(dir)) {
    if (ignored.has(entry)) continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) yield* walkFiles(full, options);
    else if (exts.some((ext) => entry.endsWith(ext))) yield full;
  }
}
