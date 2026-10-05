import { describe, expect, it } from 'vitest';

import { requestDurationRecorder } from './request-metrics';

describe('requestDurationRecorder', () => {
  it('builds nothing when no endpoint is configured', () => {
    expect(requestDurationRecorder('', 'host')).toBeUndefined();
  });

  it('builds a recorder that accepts observations when an endpoint is configured', () => {
    const record = requestDurationRecorder('http://127.0.0.1:1/api/v1/otlp/v1/metrics', 'host');
    expect(record).toBeTypeOf('function');
    record?.(0.1, {
      'http.request.method': 'GET',
      'http.route': '/',
      'http.response.status_code': 200,
    });
  });
});
