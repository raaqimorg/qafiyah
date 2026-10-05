import { Glob } from 'bun';

const DASHBOARD_DIR = `${import.meta.dir}/../../apps/observability/grafana/dashboards`;
const UIDS = ['elasticsearch', 'health', 'latency', 'slow-queries'] as const;
const WAIT_MS = 90_000;
const POLL_MS = 3_000;

export type ObservabilityResult = {
  readonly note: string;
  readonly url: string;
  readonly ms: number;
  readonly failure: string | null;
};

type Vector = readonly {
  readonly metric: Readonly<Record<string, string>>;
  readonly value: readonly [number, string];
}[];

type Grafana = { readonly url: string; readonly headers: Readonly<Record<string, string>> };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function panelExpressions(panels: unknown): string[] {
  if (!Array.isArray(panels)) return [];
  return panels.flatMap((panel: unknown) => {
    if (!isRecord(panel)) return [];
    const targets: unknown = panel['targets'];
    const own = Array.isArray(targets)
      ? targets.flatMap((target: unknown) =>
          isRecord(target) && typeof target['expr'] === 'string' ? [target['expr']] : []
        )
      : [];
    return [...own, ...panelExpressions(panel['panels'])];
  });
}

export function dashboardExpressions(dashboard: unknown): string[] {
  return isRecord(dashboard) ? panelExpressions(dashboard['panels']) : [];
}

export function substituteVariables(expr: string): string {
  return expr
    .replaceAll('$__rate_interval', '1m')
    .replaceAll('$__range', '1h')
    .replaceAll('$route', '.+')
    .replaceAll('$status', '.+');
}

async function query(
  grafana: Grafana,
  expr: string
): Promise<{ vector: Vector; error: string | null }> {
  const url = `${grafana.url}/api/datasources/proxy/uid/prometheus/api/v1/query?query=${encodeURIComponent(expr)}`;
  const response = await fetch(url, { headers: grafana.headers });
  const body = (await response.json()) as {
    status: string;
    error?: string;
    data?: { result: Vector };
  };
  if (body.status !== 'success') {
    return { vector: [], error: body.error ?? `status ${response.status}` };
  }
  return { vector: body.data?.result ?? [], error: null };
}

async function eventually(
  note: string,
  url: string,
  attempt: () => Promise<string | null>
): Promise<ObservabilityResult> {
  const started = performance.now();
  let failure: string | null = 'never ran';
  while (performance.now() - started < WAIT_MS) {
    failure = await attempt().catch(String);
    if (failure === null) break;
    await Bun.sleep(POLL_MS);
  }
  return { note, url, ms: performance.now() - started, failure };
}

function positive(grafana: Grafana, note: string, expr: string): Promise<ObservabilityResult> {
  return eventually(note, expr, async () => {
    const { vector, error } = await query(grafana, expr);
    if (error !== null) return error;
    const raw = vector[0]?.value[1];
    return Number(raw ?? Number.NaN) > 0
      ? null
      : `expected a positive value, got ${raw ?? 'no data'}`;
  });
}

async function everyTargetUp(grafana: Grafana): Promise<string | null> {
  const response = await fetch(
    `${grafana.url}/api/datasources/proxy/uid/prometheus/api/v1/targets`,
    { headers: grafana.headers }
  );
  const body = (await response.json()) as {
    data: {
      activeTargets: readonly {
        labels: Readonly<Record<string, string>>;
        health: string;
        lastError: string;
      }[];
    };
  };
  const down = body.data.activeTargets.filter((target) => target.health !== 'up');
  return down.length === 0
    ? null
    : down
        .map(
          (target) =>
            `${target.labels['job']} ${target.labels['instance']}: ${target.health} ${target.lastError}`
        )
        .join('; ');
}

async function dashboardsProvisioned(grafana: Grafana): Promise<string | null> {
  const response = await fetch(`${grafana.url}/api/search?type=dash-db`, {
    headers: grafana.headers,
  });
  const found = ((await response.json()) as readonly { uid: string }[])
    .map((dashboard) => dashboard.uid)
    .sort();
  return UIDS.every((uid) => found.includes(uid)) ? null : `found ${found.join(', ')}`;
}

async function everyQueryEvaluates(grafana: Grafana, file: string): Promise<ObservabilityResult> {
  const started = performance.now();
  const dashboard: unknown = JSON.parse(await Bun.file(`${DASHBOARD_DIR}/${file}`).text());
  const errors: string[] = [];
  for (const expr of dashboardExpressions(dashboard)) {
    const { error } = await query(grafana, substituteVariables(expr));
    if (error !== null) errors.push(`${expr}: ${error}`);
  }
  return {
    note: `every query in ${file} evaluates`,
    url: file,
    ms: performance.now() - started,
    failure: errors.length === 0 ? null : errors.join('\n'),
  };
}

export async function observabilityChecks(
  url: string,
  password: string
): Promise<readonly ObservabilityResult[]> {
  const grafana: Grafana = {
    url,
    headers: { Authorization: `Basic ${btoa(`admin:${password}`)}` },
  };
  const results: ObservabilityResult[] = [
    await eventually('grafana has the four dashboards', `${url}/api/search`, () =>
      dashboardsProvisioned(grafana)
    ),
    await eventually('every prometheus target is up', 'api/v1/targets', () =>
      everyTargetUp(grafana)
    ),
    await positive(
      grafana,
      'api requests are recorded',
      'histogram_count(sum(http_server_request_duration_seconds{job="api"}))'
    ),
    await positive(
      grafana,
      'website renders are pushed',
      'histogram_count(sum(http_server_request_duration_seconds{job="web"}))'
    ),
    await positive(
      grafana,
      'elasticsearch searches are recorded',
      'histogram_count(sum(db_client_operation_duration_seconds))'
    ),
    await positive(grafana, 'postgres is up for the exporter', 'max(pg_up)'),
    await positive(
      grafana,
      'statement text joins its timing',
      'count(pg_stat_statements_seconds_total * on (queryid) group_left (query) max by (queryid, query) (pg_stat_statements_query_id))'
    ),
    await positive(
      grafana,
      'elasticsearch is reachable and not red',
      'min(elasticsearch_clusterinfo_up) * (1 - max(elasticsearch_cluster_health_status{color="red"}))'
    ),
    await positive(
      grafana,
      'both health probes succeed',
      'min(probe_success) * count(probe_success) - 1'
    ),
  ];
  for await (const file of new Glob('*.json').scan(DASHBOARD_DIR)) {
    results.push(await everyQueryEvaluates(grafana, file));
  }
  return results;
}
