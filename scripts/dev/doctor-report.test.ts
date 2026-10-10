import { describe, expect, test } from 'bun:test';

import { blocksDev, formatReport, hasFailure } from './doctor-report';

import type { CheckOutcome } from './doctor-checks';

const plain = (text: string) => text;
const STYLE = { green: plain, red: plain, yellow: plain, dim: plain, bold: plain };

const ok = (group: CheckOutcome['group'], name: string): CheckOutcome => ({
  group,
  name,
  status: 'ok',
  detail: '1.0.0',
});

describe('blocksDev', () => {
  test('stops bun run dev only for a failure in the run group', () => {
    const commitFailure: CheckOutcome = { ...ok('commit', 'hadolint'), status: 'fail' };
    const runFailure: CheckOutcome = { ...ok('run', 'Bash'), status: 'fail' };
    expect(blocksDev([ok('run', 'Bun'), commitFailure])).toBe(false);
    expect(blocksDev([runFailure, commitFailure])).toBe(true);
  });

  test('lets a warning through', () => {
    expect(blocksDev([{ ...ok('run', 'Rust toolchain'), status: 'warn' }])).toBe(false);
  });
});

describe('hasFailure', () => {
  test('counts failures in the run and commit groups, not optional ones', () => {
    expect(hasFailure([{ ...ok('commit', 'hadolint'), status: 'fail' }])).toBe(true);
    expect(hasFailure([{ ...ok('optional', 'GitHub CLI'), status: 'fail' }])).toBe(false);
  });
});

describe('formatReport', () => {
  test('groups the checks under their headings, in order', () => {
    const report = formatReport([ok('commit', 'ShellCheck'), ok('run', 'Bun')], STYLE);
    expect(report.indexOf('To run the site')).toBeLessThan(report.indexOf('To commit and push'));
    expect(report).not.toContain('Optional');
  });

  test('marks each status and prints the fix under a problem', () => {
    const report = formatReport(
      [
        ok('run', 'Bun'),
        {
          group: 'run',
          name: 'Bash',
          status: 'fail',
          detail: '3.2.57 (needs 4)',
          fix: 'brew install bash',
        },
        {
          group: 'run',
          name: 'Rust toolchain',
          status: 'warn',
          detail: '1.99.0 not installed yet',
        },
      ],
      STYLE
    );
    expect(report).toContain('✓ Bun');
    expect(report).toContain('✗ Bash');
    expect(report).toContain('⚠ Rust toolchain');
    expect(report).toContain('fix: brew install bash');
  });
});
