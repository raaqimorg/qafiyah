'use client';

import { SEARCH_TEXTS } from '@/lib/constants/copy';
import { cn } from '@/lib/utils';

type Props = {
  readonly searchHistory: string[];
  readonly highlightedIndex: number;
  readonly listboxId: string;
  readonly onSelect: (query: string) => void;
  readonly onRemove: (query: string) => void;
  readonly onClear: () => void;
};

export function SearchHistory({
  searchHistory,
  highlightedIndex,
  listboxId,
  onSelect,
  onRemove,
  onClear,
}: Props) {
  return (
    <div className="rounded-md border border-border bg-surface-raised shadow-none">
      <div className="flex justify-between px-3 py-2">
        <span> {SEARCH_TEXTS.searchHistory}</span>
        <button type="button" onClick={onClear}>
          {SEARCH_TEXTS.deleteSearchHistory}{' '}
        </button>
      </div>
      <ul id={listboxId} role="listbox" className="max-h-30 overflow-y-auto">
        {searchHistory.map((query, index) => (
          <li
            id={`${listboxId}-option-${index}`}
            role="option"
            aria-selected={index === highlightedIndex}
            key={query}
            className={cn(
              'flex items-center justify-between rounded-md px-3 py-2 transition-colors hover:bg-surface-hover',
              index === highlightedIndex && 'bg-surface-hover'
            )}
          >
            <button
              type="button"
              className="flex-1 text-right text-base text-text-muted"
              onClick={() => onSelect(query)}
            >
              {query}
            </button>
            <button
              type="button"
              aria-label={`Remove ${query}`}
              className="px-3 text-text-muted hover:text-text"
              onClick={() => onRemove(query)}
            >
              ×
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
