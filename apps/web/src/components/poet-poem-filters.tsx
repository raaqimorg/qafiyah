import { useRef, useState } from 'react';

import { Select } from '@/components/ui-extended/select';
import { SEARCH_TEXTS } from '@/lib/constants/copy';
import {
  METERS_NOUN_FORMS,
  RHYMES_NOUN_FORMS,
  THEMES_NOUN_FORMS,
  toSelectOptions,
} from '@/lib/constants/taxonomy-data';
import {
  hasPoetFilters,
  POET_FACET_LIST,
  type PoetFilterKey,
  type PoetFilterSelection,
  samePoetFilters,
} from '@/lib/poet-filters';

import type { PoetFacets } from '@/lib/api/result-types';
import type { ArabicNounForms } from '@/lib/arabic';
import type React from 'react';

type FilterField = {
  readonly key: PoetFilterKey;
  readonly label: string;
  readonly nounForms: ArabicNounForms;
};

const FILTERS: readonly FilterField[] = [
  { key: 'meter', label: SEARCH_TEXTS.metersLabel, nounForms: METERS_NOUN_FORMS },
  { key: 'theme', label: SEARCH_TEXTS.themesLabel, nounForms: THEMES_NOUN_FORMS },
  { key: 'rhyme', label: SEARCH_TEXTS.rhymesLabel, nounForms: RHYMES_NOUN_FORMS },
];

const CLEAR_ALL_LABEL = 'مسح الكل';

type Props = {
  readonly facets: PoetFacets;
  readonly selected: PoetFilterSelection;
  readonly clearHref: string;
  readonly onApply: (next: PoetFilterSelection) => void;
  readonly onClearClick: (event: React.MouseEvent<HTMLAnchorElement>) => void;
};

export function PoetPoemFilters({ facets, selected, clearHref, onApply, onClearClick }: Props) {
  const [draft, setDraft] = useState<PoetFilterSelection | null>(null);
  const openRef = useRef<PoetFilterKey | null>(null);
  const isCancelledRef = useRef(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const shown = draft ?? selected;

  const applyFilters = (next: PoetFilterSelection) => {
    if (!samePoetFilters(next, selected)) onApply(next);
  };

  const handleChange = (key: PoetFilterKey, value: string | string[]) => {
    const next: PoetFilterSelection = {
      ...shown,
      [key]: typeof value === 'string' ? [value] : value,
    };
    if (openRef.current === key) {
      setDraft(next);
      return;
    }
    setDraft(null);
    applyFilters(next);
  };

  const handleOpenChange = (key: PoetFilterKey, isOpen: boolean) => {
    openRef.current = isOpen ? key : null;
    if (isOpen) return;
    const isCancelled = isCancelledRef.current;
    isCancelledRef.current = false;
    setDraft(null);
    if (!isCancelled && draft !== null) applyFilters(draft);
  };

  const handleEscape = () => {
    isCancelledRef.current = true;
  };

  const handleClearClick = (event: React.MouseEvent<HTMLAnchorElement>) => {
    onClearClick(event);
    if (event.defaultPrevented) {
      rootRef.current?.querySelector<HTMLElement>('[role="combobox"]')?.focus();
    }
  };

  return (
    <div ref={rootRef} className="flex w-full flex-col items-start gap-4">
      <div className="grid w-full grid-cols-2 gap-4 md:grid-cols-3 md:gap-6">
        {FILTERS.map(({ key, label, nounForms }) => (
          <Select
            key={key}
            label={label}
            options={toSelectOptions(facets[POET_FACET_LIST[key]])}
            value={shown[key]}
            placeholderNounForms={nounForms}
            onChange={(value) => handleChange(key, value)}
            onOpenChange={(isOpen) => handleOpenChange(key, isOpen)}
            onEscape={handleEscape}
            placeholder={SEARCH_TEXTS.allPlaceholder}
            allOptionLabel={SEARCH_TEXTS.allPlaceholder}
            multiple={true}
            sortOptions={false}
            showCounts={true}
          />
        ))}
      </div>
      {hasPoetFilters(selected) && (
        <a
          href={clearHref}
          onClick={handleClearClick}
          className="rounded-sm text-base text-text-subtle underline underline-offset-4 focus-ring transition-colors hover:text-text"
        >
          {CLEAR_ALL_LABEL}
        </a>
      )}
    </div>
  );
}
