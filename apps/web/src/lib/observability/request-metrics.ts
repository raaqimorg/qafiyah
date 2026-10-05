import { OTLPMetricExporter } from '@opentelemetry/exporter-metrics-otlp-proto';
import { resourceFromAttributes } from '@opentelemetry/resources';
import {
  AggregationType,
  MeterProvider,
  PeriodicExportingMetricReader,
} from '@opentelemetry/sdk-metrics';

import type { RecordDuration } from './request-timing';

const REQUEST_DURATION = 'http.server.request.duration';
const EXPORT_INTERVAL_MS = 15_000;

export function requestDurationRecorder(
  endpoint: string,
  instanceId: string
): RecordDuration | undefined {
  if (endpoint === '') return undefined;
  const provider = new MeterProvider({
    resource: resourceFromAttributes({
      'service.name': 'web',
      'service.instance.id': instanceId,
    }),
    views: [
      {
        instrumentName: REQUEST_DURATION,
        aggregation: { type: AggregationType.EXPONENTIAL_HISTOGRAM },
      },
    ],
    readers: [
      new PeriodicExportingMetricReader({
        exporter: new OTLPMetricExporter({ url: endpoint }),
        exportIntervalMillis: EXPORT_INTERVAL_MS,
      }),
    ],
  });
  const histogram = provider.getMeter('qafiyah-web').createHistogram(REQUEST_DURATION, {
    unit: 's',
    description: 'Time from the request reaching Astro to the end of its response body',
  });
  return (seconds, attributes) => histogram.record(seconds, attributes);
}
