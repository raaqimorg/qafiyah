import { getContainerRenderer } from '@astrojs/react/container-renderer';
import { experimental_AstroContainer as AstroContainer } from 'astro/container';
import { loadRenderers } from 'astro:container';
import { beforeEach, describe, expect, it, vi } from 'vitest';

beforeEach(() => {
  vi.resetModules();
  vi.doUnmock('@/lib/constants/config');
});

const POSTHOG_SCRIPT = /<script[^>]+src="[^"]*posthog[^"]*"/;

async function renderLayout(isRelease: boolean): Promise<string> {
  vi.doMock('@/lib/constants/config', async (importOriginal) => ({
    ...(await importOriginal<typeof import('@/lib/constants/config')>()),
    isRelease,
  }));
  const { default: Layout } = await import('./layout.astro');
  const container = await AstroContainer.create({
    renderers: await loadRenderers([getContainerRenderer()]),
  });
  return await container.renderToString(Layout);
}

describe('Layout', () => {
  it('loads PostHog in a release build', async () => {
    expect(await renderLayout(true)).toMatch(POSTHOG_SCRIPT);
  });

  it('leaves PostHog out of a build without a release', async () => {
    expect(await renderLayout(false)).not.toMatch(POSTHOG_SCRIPT);
  });
});
