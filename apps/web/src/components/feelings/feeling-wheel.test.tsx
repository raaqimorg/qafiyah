import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import { buildFeelingTree } from '@/lib/feelings/feeling-tree';

import { FeelingWheel } from './feeling-wheel';

const TREE = buildFeelingTree([
  { slug: 'alhuzn', name: 'الحزن', verseCount: 3 },
  { slug: 'alwahsha', name: 'الوحشة', verseCount: 3, parentSlug: 'alhuzn' },
  { slug: 'alsurur', name: 'السرور', verseCount: 2 },
]);

describe('FeelingWheel', () => {
  it('links every feeling to its page and the trunk to the wheel', () => {
    const html = renderToStaticMarkup(<FeelingWheel tree={TREE} path={[]} />);
    expect(html).toContain('href="/feelings/alhuzn"');
    expect(html).toContain('href="/feelings/alsurur"');
    expect(html).toContain('href="/feelings"');
  });

  it('marks the chosen feeling as the current page', () => {
    const html = renderToStaticMarkup(<FeelingWheel tree={TREE} path={[0, 0]} />);
    expect(html).toContain('href="/feelings/alwahsha" aria-current="page"');
    expect(html).not.toContain('href="/feelings/alhuzn" aria-current="page"');
  });
});
