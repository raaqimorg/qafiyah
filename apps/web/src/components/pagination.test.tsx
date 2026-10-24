import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import { derivePagination } from '@/lib/pagination';

import { Pagination } from './pagination';

const pageUrl = (cursor: string) => `/meters/altawil?cursor=${cursor}`;

function render(next: string | null, prev: string | null): string {
  return renderToStaticMarkup(<Pagination {...derivePagination({ next, prev }, pageUrl)} />);
}

describe('Pagination', () => {
  it('renders nothing when the list fits on one page', () => {
    expect(render(null, null)).toBe('');
  });

  it('links to the next page and disables the previous one on the first page', () => {
    const html = render('n', null);
    expect(html).toContain('<a href="/meters/altawil?cursor=n" rel="next"');
    expect(html).toContain('aria-disabled="true">السابق</span>');
    expect(html).not.toContain('rel="prev"');
  });

  it('links both ways from a middle page', () => {
    const html = render('n', 'p');
    expect(html).toContain('rel="next"');
    expect(html).toContain('<a href="/meters/altawil?cursor=p" rel="prev"');
  });

  it('disables the next link on the last page', () => {
    const html = render(null, 'p');
    expect(html).toContain('aria-disabled="true">التالي</span>');
    expect(html).not.toContain('rel="next"');
  });
});
