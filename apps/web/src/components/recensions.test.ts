import { getContainerRenderer } from '@astrojs/react/container-renderer';
import { experimental_AstroContainer as AstroContainer } from 'astro/container';
import { loadRenderers } from 'astro:container';
import { describe, expect, it } from 'vitest';

import Recensions from './recensions.astro';

const SOURCE = 'رواية الأنباري، بضبط فيصل المنصور';

async function render(recensions: unknown[]) {
  const container = await AstroContainer.create({
    renderers: await loadRenderers([getContainerRenderer()]),
  });
  return await container.renderToString(Recensions, { props: { recensions } });
}

describe('Recensions', () => {
  it('names the source of a reading next to its verse count', async () => {
    const html = await render([{ title: 'قفا نبك', slug: 'pxyk', verseCount: 94, source: SOURCE }]);
    expect(html).toContain(SOURCE);
  });

  it('shows only the verse count when a reading has no source', async () => {
    const html = await render([{ title: 'قفا نبك', slug: 'pxyk', verseCount: 94 }]);
    expect(html).toContain('pxyk');
    expect(html).not.toContain('رواية الأنباري');
  });
});
