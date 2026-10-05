import { describe, expect, test } from 'bun:test';

import { dashboardQueries, substituteVariables } from './observability';

describe('dashboardQueries', () => {
  test('collects every target with its datasource, rows included, and skips text panels', () => {
    const dashboard = {
      panels: [
        { type: 'text', options: { content: 'x' } },
        {
          type: 'table',
          datasource: { type: 'prometheus', uid: 'prometheus' },
          targets: [{ expr: 'up' }, { expr: 'pg_up', datasource: { uid: 'prometheus' } }],
        },
        {
          type: 'row',
          panels: [
            {
              type: 'logs',
              datasource: { type: 'loki', uid: 'loki' },
              targets: [{ expr: '{service="web"}', datasource: { uid: 'loki' } }],
            },
          ],
        },
      ],
    };
    expect(dashboardQueries(dashboard)).toEqual([
      { datasource: 'prometheus', expr: 'up' },
      { datasource: 'prometheus', expr: 'pg_up' },
      { datasource: 'loki', expr: '{service="web"}' },
    ]);
  });

  test('a target with no datasource of its own or on its panel goes to Prometheus', () => {
    expect(dashboardQueries({ panels: [{ type: 'stat', targets: [{ expr: 'up' }] }] })).toEqual([
      { datasource: 'prometheus', expr: 'up' },
    ]);
  });

  test('returns nothing for a value that is not a dashboard', () => {
    expect(dashboardQueries(null)).toEqual([]);
    expect(dashboardQueries({ panels: 'no' })).toEqual([]);
  });
});

describe('substituteVariables', () => {
  test('replaces exactly the variables the dashboards use', () => {
    expect(
      substituteVariables(
        'rate(x{a=~"$route", b=~"$status"}[$__rate_interval]) + increase(y[$__range]) + rate(z[$__auto])'
      )
    ).toBe('rate(x{a=~".+", b=~".+"}[1m]) + increase(y[1h]) + rate(z[1m])');
  });
});
