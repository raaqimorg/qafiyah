import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import { SITE_URL } from '@/lib/constants/config';

import { FeelingVerseItem, verseCopyText } from './feeling-verse-item';

import type { FeelingVerseRow } from '@/lib/server/feelings';

const verse = (overrides: Partial<FeelingVerseRow>): FeelingVerseRow => ({
  parts: ['وَرَدَ الفِراقُ عَلَيَّ يُتلِفُ مُهجَتي', 'يا بَرحَ قَلبي بِالفِراقِ الوارِدِ'],
  position: 1,
  feeling: { name: 'الحزن', slug: 'alhuzn' },
  poem: { title: 'أجدر وأخلق أن ترن عوائدي', slug: 'btmd' },
  poet: { name: 'البحتري', slug: 'abcd', hasAvatar: false, isAnonymous: false },
  era: { name: 'عباسي', slug: 'abbasi' },
  ...overrides,
});

const render = (row: FeelingVerseRow) => renderToStaticMarkup(<FeelingVerseItem verse={row} />);

describe('FeelingVerseItem', () => {
  it('names the poet under the verse, and not the era or the poem title', () => {
    const html = render(verse({}));
    expect(html).toContain('البحتري');
    expect(html).not.toContain('عباسي');
    expect(html).not.toContain('أجدر وأخلق أن ترن عوائدي');
  });

  it('copies each half on its own line, a blank line, the poet, and the poem link', () => {
    expect(verseCopyText(verse({}))).toBe(
      [
        'وَرَدَ الفِراقُ عَلَيَّ يُتلِفُ مُهجَتي',
        'يا بَرحَ قَلبي بِالفِراقِ الوارِدِ',
        '',
        'البحتري',
        `${SITE_URL}/poems/btmd`,
      ].join('\n')
    );
  });

  it('gives the card a copy button that sits outside the poem link', () => {
    const html = render(verse({}));
    expect(html).toContain('aria-label="نسخ البيت"');
    expect(html.indexOf('data-copy-verse')).toBeGreaterThan(html.indexOf('</a>'));
  });

  it('labels every verse with its own feeling', () => {
    expect(render(verse({}))).toContain('الحزن');
    expect(render(verse({ feeling: { name: 'الأسى', slug: 'alasa' } }))).toContain('الأسى');
  });
});
