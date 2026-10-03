import type { PaginationView } from '@/lib/pagination';
import type React from 'react';

type Props = Pick<
  PaginationView,
  'hasNextPage' | 'hasPrevPage' | 'nextPageUrl' | 'prevPageUrl' | 'headerTip' | 'totalPages'
> & {
  readonly onLinkClick?: ((event: React.MouseEvent<HTMLAnchorElement>) => void) | undefined;
};

const linkClass =
  'rounded-md border border-border px-3 py-2 text-text-muted focus-ring transition-colors hover:bg-surface-hover';
const disabledClass =
  'cursor-not-allowed rounded-md border border-border px-3 py-2 text-text-subtle';

export function Pagination({
  hasNextPage,
  hasPrevPage,
  nextPageUrl,
  prevPageUrl,
  headerTip,
  totalPages,
  onLinkClick,
}: Props) {
  if (totalPages <= 1) return null;
  return (
    <nav
      className="mt-8 flex w-full flex-row-reverse items-center justify-between gap-4 text-base md:text-lg"
      aria-label="ترقيم الصفحات"
    >
      {hasNextPage ? (
        <a href={nextPageUrl} rel="next" className={linkClass} onClick={onLinkClick}>
          التالي
        </a>
      ) : (
        <span className={disabledClass} aria-disabled="true">
          التالي
        </span>
      )}
      <p className="text-base text-text-subtle">{headerTip}</p>
      {hasPrevPage ? (
        <a href={prevPageUrl} rel="prev" className={linkClass} onClick={onLinkClick}>
          السابق
        </a>
      ) : (
        <span className={disabledClass} aria-disabled="true">
          السابق
        </span>
      )}
    </nav>
  );
}
