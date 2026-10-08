import { getContainerRenderer } from '@astrojs/react/container-renderer';
import { experimental_AstroContainer as AstroContainer } from 'astro/container';
import { loadRenderers } from 'astro:container';
import { describe, expect, it } from 'vitest';

import { VIEWER_HINT_COOKIE } from '@/lib/constants/config';
import { NEW_KEY_COOKIE } from '@/lib/server/new-key-cookie';
import { STATE_COOKIE } from '@/lib/server/oauth/redirect';
import { SESSION_COOKIE } from '@/lib/server/session';

import PrivacyPage from './privacy.astro';

async function render(): Promise<string> {
  const container = await AstroContainer.create({
    renderers: await loadRenderers([getContainerRenderer()]),
  });
  return await container.renderToString(PrivacyPage, {
    request: new Request('https://qafiyah.com/privacy'),
  });
}

describe('GET /privacy', () => {
  it('names every cookie that the website sets', async () => {
    const html = await render();
    for (const cookie of [SESSION_COOKIE, STATE_COOKIE, NEW_KEY_COOKIE, VIEWER_HINT_COOKIE]) {
      expect(html).toContain(cookie);
    }
  });

  it('is an indexable page with its own canonical URL', async () => {
    const html = await render();
    expect(html).toContain('<link rel="canonical" href="http://localhost:4321/privacy"');
    expect(html).toContain('index, follow');
  });
});
