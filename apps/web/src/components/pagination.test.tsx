import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import { derivePagination } from '@/lib/pagination';

import { Pagination } from './pagination';

const pageUrl = (page: number) => `/meters/altawil?page=${page}`;

function render(page: number, totalPages: number): string {
  return renderToStaticMarkup(<Pagination {...derivePagination({ page, totalPages }, pageUrl)} />);
}

describe('Pagination', () => {
  it('renders nothing when the list fits on one page', () => {
    expect(render(1, 1)).toBe('');
  });

  it('links to the next page and disables the previous one on the first page', () => {
    const html = render(1, 3);
    expect(html).toContain('<a href="/meters/altawil?page=2" rel="next"');
    expect(html).toContain('aria-disabled="true">السابق</span>');
    expect(html).not.toContain('rel="prev"');
  });

  it('links both ways from a middle page and shows where the reader is', () => {
    const html = render(2, 3);
    expect(html).toContain('rel="next"');
    expect(html).toContain('<a href="/meters/altawil?page=1" rel="prev"');
    expect(html).toContain('صـ ٢ من ٣');
  });

  it('disables the next link on the last page', () => {
    const html = render(3, 3);
    expect(html).toContain('aria-disabled="true">التالي</span>');
    expect(html).not.toContain('rel="next"');
  });
});
