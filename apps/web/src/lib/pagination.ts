export function readCursor(url: URL): string | undefined {
  const raw = url.searchParams.get('cursor');
  return raw === null || raw === '' ? undefined : raw;
}

type Cursors = {
  readonly next: string | null;
  readonly prev: string | null;
};

export type PaginationView = {
  readonly nextUrl: string | undefined;
  readonly prevUrl: string | undefined;
};

export function derivePagination(
  page: Cursors,
  makeUrl: (cursor: string) => string
): PaginationView {
  return {
    nextUrl: page.next === null ? undefined : makeUrl(page.next),
    prevUrl: page.prev === null ? undefined : makeUrl(page.prev),
  };
}

export function reachedFirstPage(cursor: string | undefined, page: Cursors): boolean {
  return cursor !== undefined && page.prev === null;
}
