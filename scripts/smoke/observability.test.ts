import { describe, expect, test } from 'bun:test';

import { dashboardExpressions, substituteVariables } from './observability';

describe('dashboardExpressions', () => {
  test('collects every target expression, rows included, and skips text panels', () => {
    const dashboard = {
      panels: [
        { type: 'text', options: { content: 'x' } },
        { type: 'table', targets: [{ expr: 'up' }, { expr: 'pg_up' }] },
        { type: 'row', panels: [{ type: 'stat', targets: [{ expr: 'probe_success' }] }] },
      ],
    };
    expect(dashboardExpressions(dashboard)).toEqual(['up', 'pg_up', 'probe_success']);
  });

  test('returns nothing for a value that is not a dashboard', () => {
    expect(dashboardExpressions(null)).toEqual([]);
    expect(dashboardExpressions({ panels: 'no' })).toEqual([]);
  });
});

describe('substituteVariables', () => {
  test('replaces exactly the variables the dashboards use', () => {
    expect(
      substituteVariables(
        'rate(x{a=~"$route", b=~"$status"}[$__rate_interval]) + increase(y[$__range])'
      )
    ).toBe('rate(x{a=~".+", b=~".+"}[1m]) + increase(y[1h])');
  });
});
