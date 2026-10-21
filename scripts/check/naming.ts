#!/usr/bin/env bun

import { existsSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

import { reportViolations } from '../lib/report';
import { ROOT } from '../lib/root';
import { IGNORED_DIRS as BUILD_DIRS, toPosixPath } from '../lib/walk';

const IGNORED_DIRS = new Set([
  ...BUILD_DIRS,
  '.claude',
  '.github',
  '.vscode',
  'data',
  'docs',
  'graft',
  'tmp',
  'tools',
  'venv',
]);

const IGNORED_REL_PATHS = new Set(['apps/web/public', 'apps/api/migrations']);

const IGNORED_FILES = new Set(['.DS_Store']);

const VALIDATED_EXTS = new Set([
  '.ts',
  '.tsx',
  '.js',
  '.jsx',
  '.mjs',
  '.cjs',
  '.astro',
  '.json',
  '.toml',
  '.yml',
  '.yaml',
  '.css',
  '.sh',
  '.conf',
  '.sql',
]);

const ALLOWED_BASENAMES = new Set([
  'README.md',
  'LICENSE',
  'AGENTS.md',
  'CLAUDE.md',
  'TODO.md',
  'CHANGELOG.md',
  'MAINTAINERS_GUIDE.md',
  'Dockerfile',
  'Cargo.toml',
  'Cargo.lock',
]);

const SEGMENT_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const BRACKET_RE = /^\[\.{0,3}[a-z0-9]+(?:-[a-z0-9]+)*\]$/;

const isValidSegment = (segment: string): boolean =>
  SEGMENT_RE.test(segment) || BRACKET_RE.test(segment);

function shouldValidateFile(name: string): boolean {
  if (IGNORED_FILES.has(name)) return false;
  if (ALLOWED_BASENAMES.has(name)) return true;
  if (name.startsWith('.')) return true;
  const dot = name.lastIndexOf('.');
  if (dot < 0) return false;
  return VALIDATED_EXTS.has(name.slice(dot));
}

function isValidFile(name: string): boolean {
  if (ALLOWED_BASENAMES.has(name)) return true;
  const stripped = name.startsWith('.') ? name.slice(1) : name;
  const bracketed = /^(\[[^\]]*\])(.*)$/u.exec(stripped);
  if (bracketed) {
    const [, head = '', rest = ''] = bracketed;
    if (!BRACKET_RE.test(head)) return false;
    return (
      rest === '' ||
      rest
        .slice(1)
        .split('.')
        .every((segment) => isValidSegment(segment))
    );
  }
  return stripped.split('.').every((segment) => isValidSegment(segment));
}

const isRustModuleDir = (dir: string): boolean => existsSync(join(dir, 'mod.rs'));

function isValidDir(name: string): boolean {
  const stripped = name.startsWith('.') ? name.slice(1) : name;
  return stripped.split('.').every((segment) => isValidSegment(segment));
}

type Violation = { kind: 'file' | 'dir'; path: string };

const violations: Violation[] = [];

function walk(dir: string): void {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const rel = toPosixPath(relative(ROOT, full));
    const s = statSync(full);
    if (s.isDirectory()) {
      if (IGNORED_DIRS.has(entry)) continue;
      if (IGNORED_REL_PATHS.has(rel)) continue;
      if (!isValidDir(entry) && !isRustModuleDir(full)) {
        violations.push({ kind: 'dir', path: rel });
      }
      walk(full);
    } else {
      if (!shouldValidateFile(entry)) continue;
      if (!isValidFile(entry)) violations.push({ kind: 'file', path: rel });
    }
  }
}

walk(ROOT);

process.exit(
  reportViolations({
    title: 'Naming convention violations:',
    lines: violations.map((v) => `${v.kind.padEnd(4)} ${v.path}`),
    rule: [
      'Rule: every dot-separated segment of a name must be kebab-case',
      '      ([a-z0-9]+ joined by single hyphens).',
      'Allow-listed:',
      '  - Astro brackets, e.g. [slug], [page], [...rest]',
      '  - Numeric segments, e.g. 404.astro',
      `  - Docs/special files: ${[...ALLOWED_BASENAMES].join(', ')}`,
    ],
    ok: 'All file and directory names conform.',
  })
);
