export type Captured = {
  readonly status: number;
  readonly headers: Readonly<Record<string, string>>;
  readonly body: string;
};

export type Difference = {
  readonly path: string;
  readonly field: string;
  readonly a: string;
  readonly b: string;
};

export type Timing = {
  readonly path: string;
  readonly group: string;
  readonly a: readonly number[];
  readonly b: readonly number[];
};

export type GroupSummary = {
  readonly group: string;
  readonly urls: number;
  readonly samples: number;
  readonly aP50: number;
  readonly bP50: number;
  readonly aP95: number;
  readonly bP95: number;
  readonly ratio: number;
};

export const KEPT_HEADERS = ['content-type', 'cache-control', 'etag', 'location'] as const;

export function diffCaptured(
  path: string,
  a: Captured,
  b: Captured,
  statusOnly: boolean
): Difference[] {
  const differences: Difference[] = [];
  if (a.status !== b.status) {
    differences.push({ path, field: 'status', a: String(a.status), b: String(b.status) });
  }
  const names = statusOnly
    ? ['content-type']
    : [...new Set([...Object.keys(a.headers), ...Object.keys(b.headers)])].sort();
  for (const name of names) {
    if (a.headers[name] !== b.headers[name]) {
      differences.push({
        path,
        field: `header ${name}`,
        a: a.headers[name] ?? '',
        b: b.headers[name] ?? '',
      });
    }
  }
  if (!statusOnly && a.body !== b.body) {
    differences.push({ path, field: 'body', a: a.body.slice(0, 300), b: b.body.slice(0, 300) });
  }
  return differences;
}

export function maskAccountBody(body: string): string {
  return body
    .replaceAll(/"id":(?:\d+|"[^"]*")/gu, '"id":"<id>"')
    .replaceAll(/"email":"compare-[^"]*@example\.test"/gu, '"email":"<email>"')
    .replaceAll(/"value":"[^"]*"/gu, '"value":"<key>"')
    .replaceAll(/"prefix":"[^"]*"/gu, '"prefix":"<prefix>"')
    .replaceAll(/"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z"/gu, '"<time>"')
    .replaceAll(/\/account\/sessions\/[A-Za-z0-9_-]+/gu, '/account/sessions/<session>');
}

export function percentile(values: readonly number[], p: number): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((x, y) => x - y);
  const rank = Math.max(1, Math.ceil((p / 100) * sorted.length));
  return sorted[rank - 1] ?? 0;
}

export function summarize(timings: readonly Timing[]): GroupSummary[] {
  const groups = new Map<string, { urls: number; a: number[]; b: number[] }>();
  for (const timing of timings) {
    const entry = groups.get(timing.group) ?? { urls: 0, a: [], b: [] };
    entry.urls += 1;
    entry.a.push(...timing.a);
    entry.b.push(...timing.b);
    groups.set(timing.group, entry);
  }
  return [...groups.entries()]
    .map(([group, entry]) => {
      const aP50 = percentile(entry.a, 50);
      const bP50 = percentile(entry.b, 50);
      return {
        group,
        urls: entry.urls,
        samples: entry.a.length,
        aP50,
        bP50,
        aP95: percentile(entry.a, 95),
        bP95: percentile(entry.b, 95),
        ratio: aP50 === 0 ? 0 : Math.round((bP50 / aP50) * 100) / 100,
      };
    })
    .sort((x, y) => x.group.localeCompare(y.group));
}
