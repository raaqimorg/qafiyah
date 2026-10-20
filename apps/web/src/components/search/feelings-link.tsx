import { Sparkles } from 'lucide-react';

import { SEARCH_TEXTS } from '@/lib/constants/copy';
import { feelingUrl } from '@/lib/urls';

export function FeelingsLink({ placeholder = false }: { readonly placeholder?: boolean }) {
  return (
    <a
      href={feelingUrl()}
      tabIndex={placeholder ? -1 : undefined}
      aria-hidden={placeholder ? 'true' : undefined}
      className="flex h-12 shrink-0 items-center justify-center gap-2 rounded-md border border-border bg-transparent px-4 text-base whitespace-nowrap text-text-subtle focus-ring transition-colors hover:bg-surface-hover hover:text-text-muted"
    >
      <Sparkles size={16} className="h-4 w-4 text-highlight" />
      <span>{SEARCH_TEXTS.feelingsLabel}</span>
    </a>
  );
}
