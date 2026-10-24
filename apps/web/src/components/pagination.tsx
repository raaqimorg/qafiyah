import type { PaginationView } from '@/lib/pagination';
import type React from 'react';

type Props = PaginationView & {
  readonly onLinkClick?: ((event: React.MouseEvent<HTMLAnchorElement>) => void) | undefined;
};

const linkClass =
  'rounded-md border border-border px-3 py-2 text-text-muted focus-ring transition-colors hover:bg-surface-hover';
const disabledClass =
  'cursor-not-allowed rounded-md border border-border px-3 py-2 text-text-subtle';

export function Pagination({ nextUrl, prevUrl, onLinkClick }: Props) {
  if (nextUrl === undefined && prevUrl === undefined) return null;
  return (
    <nav
      className="mt-8 flex w-full flex-row-reverse items-center justify-between gap-4 text-base md:text-lg"
      aria-label="ترقيم الصفحات"
    >
      {nextUrl === undefined ? (
        <span className={disabledClass} aria-disabled="true">
          التالي
        </span>
      ) : (
        <a href={nextUrl} rel="next" className={linkClass} onClick={onLinkClick}>
          التالي
        </a>
      )}
      {prevUrl === undefined ? (
        <span className={disabledClass} aria-disabled="true">
          السابق
        </span>
      ) : (
        <a href={prevUrl} rel="prev" className={linkClass} onClick={onLinkClick}>
          السابق
        </a>
      )}
    </nav>
  );
}
