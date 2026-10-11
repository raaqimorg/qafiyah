const ALLOWED_TARGETS: ReadonlySet<string> = new Set(['amudi']);
const NUMERIC_ID = /^\d+$/;

export type Transition = {
  readonly id: string;
  readonly current: string;
  readonly proposed: string;
};

export function parseTransitions(csv: string): readonly Transition[] {
  return csv
    .split('\n')
    .slice(1)
    .filter((line) => line.trim().length > 0)
    .map((line) => {
      const [id, , current, proposed] = line.split(',') as [string, string, string, string];
      return { id, current, proposed };
    });
}

export function buildUpdateStatements(transitions: readonly Transition[]): readonly string[] {
  const byTarget = new Map<string, string[]>();

  for (const transition of transitions) {
    if (!NUMERIC_ID.test(transition.id)) {
      throw new Error(`refusing a non-numeric poem id: ${transition.id}`);
    }
    if (!ALLOWED_TARGETS.has(transition.proposed)) {
      throw new Error(`refusing an unknown target label: ${transition.proposed}`);
    }
    const ids = byTarget.get(transition.proposed) ?? [];
    ids.push(transition.id);
    byTarget.set(transition.proposed, ids);
  }

  return [...byTarget].map(
    ([target, ids]) =>
      `UPDATE poems SET poem_type_id = (SELECT id FROM poem_types WHERE slug = '${target}') WHERE id IN (${ids.join(', ')});`
  );
}

const DEFAULT_OUTPUT = 'reports/corpus/apply-verdicts.sql';

async function main(): Promise<void> {
  const input = Bun.argv[2];
  if (input === undefined) {
    console.error('usage: apply-verdicts.ts <transitions.csv> [out.sql]');
    process.exit(1);
  }
  const output = Bun.argv[3] ?? DEFAULT_OUTPUT;

  const transitions = parseTransitions(await Bun.file(input).text());
  const statements = buildUpdateStatements(transitions);

  const counts = new Map<string, number>();
  for (const transition of transitions) {
    const key = `${transition.current} -> ${transition.proposed}`;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }

  await Bun.write(output, ['BEGIN;', ...statements, 'COMMIT;', ''].join('\n'));

  for (const [key, count] of [...counts].sort((left, right) => right[1] - left[1])) {
    console.log(`${key}: ${count}`);
  }
  console.log(`wrote ${statements.length} statements for ${transitions.length} poems to ${output}`);
}

if (import.meta.main) await main();
