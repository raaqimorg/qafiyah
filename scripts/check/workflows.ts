#!/usr/bin/env bun

import { runExternalLinter } from '../lib/external-linter';
import { listTrackedFiles } from '../lib/tracked-files';

const isWorkflow = (path: string): boolean =>
  path.startsWith('.github/workflows/') && (path.endsWith('.yml') || path.endsWith('.yaml'));

const tracked = listTrackedFiles();
if (tracked.isErr()) {
  console.error(tracked.error);
  process.exit(1);
}

process.exit(
  runExternalLinter({
    tool: 'actionlint',
    args: [],
    files: tracked.value.filter(isWorkflow),
    installHint: 'brew install actionlint',
  })
);
