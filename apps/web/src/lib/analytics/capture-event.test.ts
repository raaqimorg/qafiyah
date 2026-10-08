import { beforeEach, describe, expect, it, vi } from 'vitest';

const posthog = vi.hoisted(() => ({ __loaded: false, capture: vi.fn() }));
vi.mock('posthog-js', () => ({ posthog }));

import { captureEvent } from './capture-event';

beforeEach(() => {
  posthog.capture.mockReset();
});

describe('captureEvent', () => {
  it('sends nothing before PostHog starts', () => {
    posthog.__loaded = false;
    captureEvent('random_poem_requested');
    expect(posthog.capture).not.toHaveBeenCalled();
  });

  it('sends the event once PostHog has started', () => {
    posthog.__loaded = true;
    captureEvent('random_poem_requested', { source: 'nav' });
    expect(posthog.capture).toHaveBeenCalledWith('random_poem_requested', { source: 'nav' });
  });
});
