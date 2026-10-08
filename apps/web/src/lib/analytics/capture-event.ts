import { posthog } from 'posthog-js';

import type { Properties } from 'posthog-js';

export function captureEvent(name: string, properties?: Properties): void {
  if (!posthog.__loaded) return;
  posthog.capture(name, properties);
}
