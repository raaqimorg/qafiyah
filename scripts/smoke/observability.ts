import { Glob } from 'bun';

const DASHBOARD_DIR = `${import.meta.dir}/../../apps/observability/grafana/dashboards`;
const UIDS = [
  'edge',
  'elasticsearch',
  'health',
  'latency',
  'logs',
  'resources',
  'slow-queries',
] as const;
const RULE_UIDS = [
  'accounts-backup',
  'api-down',
  'container-memory',
  'disk',
  'elasticsearch-unhealthy',
  'error-rate',
  'latency-api',
  'latency-search',
  'latency-web',
  'pipeline-down',
  'postgres-down',
  'web-down',
] as const;
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

export type DashboardQuery = { readonly datasource: string; readonly expr: string };

function datasourceUid(holder: Record<string, unknown>): string | undefined {
  const datasource: unknown = holder['datasource'];
  return isRecord(datasource) && typeof datasource['uid'] === 'string'
    ? datasource['uid']
    : undefined;
}

function panelQueries(panels: unknown): DashboardQuery[] {
  if (!Array.isArray(panels)) return [];
  return panels.flatMap((panel: unknown) => {
    if (!isRecord(panel)) return [];
    const targets: unknown = panel['targets'];
    const own = Array.isArray(targets)
      ? targets.flatMap((target: unknown) =>
          isRecord(target) && typeof target['expr'] === 'string'
            ? [
                {
                  datasource: datasourceUid(target) ?? datasourceUid(panel) ?? 'prometheus',
                  expr: target['expr'],
                },
              ]
            : []
        )
      : [];
    return [...own, ...panelQueries(panel['panels'])];
  });
}

export function dashboardQueries(dashboard: unknown): DashboardQuery[] {
  return isRecord(dashboard) ? panelQueries(dashboard['panels']) : [];
}

export type RuleQuery = { readonly uid: string; readonly expr: string };

export function ruleQueries(rules: unknown): RuleQuery[] {
  if (!Array.isArray(rules)) return [];
  return rules.flatMap((rule: unknown) => {
    if (!isRecord(rule) || typeof rule['uid'] !== 'string' || !Array.isArray(rule['data'])) {
      return [];
    }
    const uid = rule['uid'];
    return rule['data'].flatMap((step: unknown) => {
      if (!isRecord(step) || step['datasourceUid'] !== 'prometheus' || !isRecord(step['model'])) {
        return [];
      }
      const expr = step['model']['expr'];
      return typeof expr === 'string' ? [{ uid, expr }] : [];
    });
  });
}

export function substituteVariables(expr: string): string {
  return expr
    .replaceAll('$__rate_interval', '1m')
    .replaceAll('$__auto', '1m')
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

async function lokiQuery(
  grafana: Grafana,
  expr: string
): Promise<{ vector: Vector; error: string | null }> {
  const end = Date.now() * 1_000_000;
  const start = end - 3_600_000 * 1_000_000;
  const url = `${grafana.url}/api/datasources/proxy/uid/loki/loki/api/v1/query_range?query=${encodeURIComponent(expr)}&start=${start}&end=${end}&step=3600&limit=1`;
  const response = await fetch(url, { headers: grafana.headers });
  const body = (await response.json()) as {
    status?: string;
    error?: string;
    message?: string;
    data?: { result: readonly { metric?: Record<string, string>; values?: [string, string][] }[] };
  };
  if (body.status !== 'success') {
    return { vector: [], error: body.error ?? body.message ?? `status ${response.status}` };
  }
  const vector = (body.data?.result ?? []).map((series) => {
    const last = series.values?.at(-1) ?? [String(Date.now() / 1000), '0'];
    return { metric: series.metric ?? {}, value: [Number(last[0]), last[1]] as const };
  });
  return { vector, error: null };
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

function positive(
  grafana: Grafana,
  note: string,
  expr: string,
  datasource: 'prometheus' | 'loki' = 'prometheus'
): Promise<ObservabilityResult> {
  return eventually(note, expr, async () => {
    const { vector, error } = await (datasource === 'loki'
      ? lokiQuery(grafana, expr)
      : query(grafana, expr));
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

async function everyAlertRuleEvaluates(grafana: Grafana): Promise<ObservabilityResult> {
  const started = performance.now();
  const response = await fetch(`${grafana.url}/api/v1/provisioning/alert-rules`, {
    headers: grafana.headers,
  });
  const queries = ruleQueries(await response.json());
  const found = queries.map((rule) => rule.uid).sort();
  const errors: string[] = RULE_UIDS.every((uid) => found.includes(uid))
    ? []
    : [`found rules ${found.join(', ')}`];
  for (const { uid, expr } of queries) {
    const { error } = await query(grafana, expr);
    if (error !== null) errors.push(`${uid}: ${error}`);
  }
  return {
    note: 'every alert rule is provisioned and its query evaluates',
    url: '/api/v1/provisioning/alert-rules',
    ms: performance.now() - started,
    failure: errors.length === 0 ? null : errors.join('\n'),
  };
}

async function everyQueryEvaluates(grafana: Grafana, file: string): Promise<ObservabilityResult> {
  const started = performance.now();
  const dashboard: unknown = JSON.parse(await Bun.file(`${DASHBOARD_DIR}/${file}`).text());
  const errors: string[] = [];
  for (const { datasource, expr } of dashboardQueries(dashboard)) {
    const substituted = substituteVariables(expr);
    const { error } = await (datasource === 'loki'
      ? lokiQuery(grafana, substituted)
      : query(grafana, substituted));
    if (error !== null) errors.push(`${datasource}: ${expr}: ${error}`);
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
    await eventually('grafana has every dashboard', `${url}/api/search`, () =>
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
    await positive(
      grafana,
      'container memory and limits arrive',
      'count(container_spec_memory_limit_bytes{name!=""} > 0)'
    ),
    await positive(grafana, 'host memory arrives', 'max(node_memory_MemTotal_bytes)'),
    await positive(
      grafana,
      'the website nginx log reaches loki as JSON',
      'sum(count_over_time({service="web"} |= `"source":"nginx"` | json | __error__="" | request_time >= 0 [1h]))',
      'loki'
    ),
    await everyAlertRuleEvaluates(grafana),
  ];
  for await (const file of new Glob('*.json').scan(DASHBOARD_DIR)) {
    results.push(await everyQueryEvaluates(grafana, file));
  }
  return results;
}
