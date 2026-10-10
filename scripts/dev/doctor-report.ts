import type { CheckOutcome } from './doctor-checks';

type Paint = (text: string) => string;

type Style = {
  readonly green: Paint;
  readonly red: Paint;
  readonly yellow: Paint;
  readonly dim: Paint;
  readonly bold: Paint;
};

const HEADINGS: Record<CheckOutcome['group'], string> = {
  run: 'To run the site',
  commit: 'To commit and push',
  optional: 'Optional',
};

const GROUP_ORDER = ['run', 'commit', 'optional'] as const;

const MARKS: Record<CheckOutcome['status'], (style: Style) => string> = {
  ok: (style) => style.green('✓'),
  warn: (style) => style.yellow('⚠'),
  fail: (style) => style.red('✗'),
  skip: (style) => style.dim('·'),
};

export function formatReport(outcomes: readonly CheckOutcome[], style: Style): string {
  const width = Math.max(0, ...outcomes.map((item) => item.name.length));
  const lines: string[] = [];
  for (const group of GROUP_ORDER) {
    const items = outcomes.filter((item) => item.group === group);
    if (items.length === 0) continue;
    lines.push(style.bold(HEADINGS[group]));
    for (const item of items) {
      const detail = item.status === 'ok' ? style.dim(item.detail) : item.detail;
      lines.push(`  ${MARKS[item.status](style)} ${item.name.padEnd(width)}  ${detail}`);
      if (item.fix !== undefined) {
        lines.push(`  ${' '.repeat(width + 2)}  ${style.dim(`fix: ${item.fix}`)}`);
      }
    }
  }
  return lines.join('\n');
}

export function blocksDev(outcomes: readonly CheckOutcome[]): boolean {
  return outcomes.some((item) => item.group === 'run' && item.status === 'fail');
}

export function hasFailure(outcomes: readonly CheckOutcome[]): boolean {
  return outcomes.some((item) => item.group !== 'optional' && item.status === 'fail');
}
