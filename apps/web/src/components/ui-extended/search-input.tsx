'use client';

import { Search, X } from 'lucide-react';
import {
  type ChangeEventHandler,
  type KeyboardEventHandler,
  type MouseEventHandler,
  useId,
  useRef,
  useState,
} from 'react';

import { SearchHistory } from '@/components/ui-extended/search-history';
import { IconButton } from '@/components/ui/icon-button';
import { Input } from '@/components/ui/input';
import { SEARCH_TEXTS } from '@/lib/constants/copy';
import { cn } from '@/lib/utils';
import { MAX_QUERY_LENGTH } from '@qafiyah/config';

type Props = {
  readonly inputValue: string;
  readonly visibleSearchHistory: readonly string[];
  readonly onSelectHistoryEntry: (entry: string) => void;
  readonly onRemoveHistoryEntry: (entry: string) => void;
  readonly onClearSearchHistory: () => void;
  readonly validationError: string | null;
  readonly searchLabel: string;
  readonly onKeyDown: KeyboardEventHandler<HTMLInputElement>;
  readonly onInputChange: ChangeEventHandler<HTMLInputElement>;
  readonly onInputBlur: () => void;
  readonly onClearInput: () => void;
  readonly onSearch: () => void;
  readonly hasQueryToShow: boolean;
  readonly placeholder: string;
};

const preventInputBlur: MouseEventHandler = (event) => {
  event.preventDefault();
};

export function SearchInput({
  inputValue,
  visibleSearchHistory,
  onSelectHistoryEntry,
  onRemoveHistoryEntry,
  onClearSearchHistory,
  validationError,
  searchLabel,
  onKeyDown,
  onInputChange,
  onInputBlur,
  onClearInput,
  onSearch,
  placeholder,
  hasQueryToShow,
}: Props) {
  const wasFocusedRef = useRef(false);

  const [isOpen, setIsOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(0);
  const listboxId = useId();
  const isListVisible = isOpen && visibleSearchHistory.length > 0;

  const handleHistoryKeyDown: KeyboardEventHandler<HTMLInputElement> = (e) => {
    if (!isListVisible) return;
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setHighlightedIndex((prev) => (prev + 1) % visibleSearchHistory.length);
        break;

      case 'ArrowUp':
        e.preventDefault();
        setHighlightedIndex(
          (prev) => (prev - 1 + visibleSearchHistory.length) % visibleSearchHistory.length
        );
        break;

      case 'Enter': {
        setIsOpen(false);
        const entry = visibleSearchHistory[highlightedIndex];
        if (entry !== undefined) {
          e.preventDefault();
          onSelectHistoryEntry(entry);
        }
        break;
      }

      case 'Escape':
        e.preventDefault();
        setIsOpen(false);
        break;
    }
  };

  const handleInputFocus = () => {
    setIsOpen(true);
    setHighlightedIndex(-1);
  };
  const handleInputMouseDown: MouseEventHandler<HTMLInputElement> = (event) => {
    wasFocusedRef.current = event.currentTarget === document.activeElement;
  };

  const handleInputClick: MouseEventHandler<HTMLInputElement> = (event) => {
    if (wasFocusedRef.current) return;
    const el = event.currentTarget;
    if (el.value.length > 0 && el.selectionStart === 0 && el.selectionEnd === 0) {
      const end = el.value.length;
      el.setSelectionRange(end, end);
    }
  };

  return (
    <div tabIndex={-1} className="w-full">
      <div tabIndex={-1} className={cn('mb-2 flex h-4 items-center justify-between')}>
        {validationError !== null && (
          <p tabIndex={-1} className={cn('text-right text-xs text-danger md:text-base')}>
            {validationError}
          </p>
        )}
      </div>
      <div tabIndex={-1} className="relative">
        <Input
          role="combobox"
          aria-expanded={isListVisible}
          aria-haspopup="listbox"
          aria-controls={listboxId}
          aria-activedescendant={
            isListVisible && highlightedIndex >= 0
              ? `${listboxId}-option-${highlightedIndex}`
              : undefined
          }
          tabIndex={0}
          placeholder={placeholder}
          maxLength={MAX_QUERY_LENGTH}
          value={inputValue}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          autoSave="off"
          spellCheck={false}
          onChange={onInputChange}
          onKeyDown={(e) => {
            handleHistoryKeyDown(e);
            if (e.defaultPrevented) return;
            if (e.key === 'Enter') setIsOpen(false);
            onKeyDown(e);
          }}
          onBlur={() => {
            setIsOpen(false);
            onInputBlur();
          }}
          onMouseDown={handleInputMouseDown}
          onClick={handleInputClick}
          onFocus={handleInputFocus}
          enterKeyHint="search"
          aria-label={searchLabel}
          className={cn(
            'h-12 rounded-none border-0 border-b border-border bg-transparent pr-14 pl-14 text-right shadow-none ring-0 transition-colors placeholder:text-text-subtle focus-visible:border-text focus-visible:ring-0 focus-visible:outline-none md:text-lg',
            { 'border-danger focus-visible:border-danger': validationError }
          )}
          dir="rtl"
        />
        {isListVisible && (
          <div
            role="presentation"
            className="absolute top-full left-0 z-50 w-full"
            onMouseDown={preventInputBlur}
          >
            <SearchHistory
              searchHistory={[...visibleSearchHistory]}
              highlightedIndex={highlightedIndex}
              listboxId={listboxId}
              onSelect={onSelectHistoryEntry}
              onRemove={onRemoveHistoryEntry}
              onClear={onClearSearchHistory}
            />
          </div>
        )}
        <IconButton
          onClick={onSearch}
          aria-label={searchLabel}
          className="absolute top-1/2 right-2 -translate-y-1/2"
        >
          <Search className="h-5 w-5" aria-hidden="true" />
        </IconButton>
        {hasQueryToShow && (
          <div tabIndex={-1} className="absolute top-1/2 left-2 flex -translate-y-1/2 items-center">
            <IconButton
              tabIndex={0}
              onMouseDown={preventInputBlur}
              onClick={onClearInput}
              aria-label={SEARCH_TEXTS.deleteSearchHistoryEntryAria}
            >
              <X className="h-4 w-4" aria-hidden="true" />
            </IconButton>
          </div>
        )}
      </div>
    </div>
  );
}
