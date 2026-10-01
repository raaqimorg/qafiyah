import { ChevronDown, ChevronUp, Filter, WholeWord } from 'lucide-react';
import { useId } from 'react';

import { Select } from '@/components/ui-extended/select';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { SEARCH_TEXTS } from '@/lib/constants/copy';
import {
  COLLECTIONS_NOUN_FORMS,
  ERAS_NOUN_FORMS,
  METERS_NOUN_FORMS,
  POEM_TYPES_NOUN_FORMS,
  RHYMES_NOUN_FORMS,
  type SelectOption,
  THEMES_NOUN_FORMS,
} from '@/lib/constants/taxonomy-data';
import { cn } from '@/lib/utils';

const labelClass = 'block text-base leading-base text-text-muted';

type MultiFilter = {
  readonly selected: readonly string[];
  readonly options: readonly SelectOption[];
  readonly onChange: (v: string | string[]) => void;
};

type Props = {
  readonly filters: {
    readonly eras: MultiFilter;
    readonly meters: MultiFilter;
    readonly poemTypes: MultiFilter;
    readonly themes: MultiFilter;
    readonly rhymes: MultiFilter;
    readonly collections: MultiFilter;
  };
  readonly wantPoems: boolean;
};

export function Filters({ filters, wantPoems }: Props) {
  const titleId = useId();
  return (
    <section aria-labelledby={titleId} className="px-1 pt-4 pb-2 md:pt-6 md:pb-4">
      <h2 id={titleId} className="mb-4 text-lg leading-tight text-text md:mb-6">
        {SEARCH_TEXTS.filtersTitle}
      </h2>
      <div className="grid grid-cols-2 gap-4 md:grid-cols-3 md:gap-6">
        <div className="flex min-w-0 flex-col items-start justify-start gap-2">
          <p className={labelClass}>{SEARCH_TEXTS.erasLabel}</p>
          <Select
            options={filters.eras.options}
            value={filters.eras.selected}
            placeholderNounForms={ERAS_NOUN_FORMS}
            onChange={filters.eras.onChange}
            placeholder={SEARCH_TEXTS.allPlaceholder}
            allOptionLabel={SEARCH_TEXTS.allPlaceholder}
            multiple={true}
            sortOptions={false}
            showCounts={true}
          />
        </div>

        {wantPoems && (
          <>
            <div className="flex min-w-0 flex-col items-start justify-start gap-2">
              <p className={labelClass}>{SEARCH_TEXTS.metersLabel}</p>
              <Select
                options={filters.meters.options}
                value={filters.meters.selected}
                placeholderNounForms={METERS_NOUN_FORMS}
                onChange={filters.meters.onChange}
                placeholder={SEARCH_TEXTS.allPlaceholder}
                allOptionLabel={SEARCH_TEXTS.allPlaceholder}
                multiple={true}
                sortOptions={false}
                showCounts={true}
              />
            </div>

            <div className="flex min-w-0 flex-col items-start justify-start gap-2">
              <p className={labelClass}>{SEARCH_TEXTS.poemTypesLabel}</p>
              <Select
                options={filters.poemTypes.options}
                value={filters.poemTypes.selected}
                placeholderNounForms={POEM_TYPES_NOUN_FORMS}
                onChange={filters.poemTypes.onChange}
                placeholder={SEARCH_TEXTS.allPlaceholder}
                allOptionLabel={SEARCH_TEXTS.allPlaceholder}
                multiple={true}
                sortOptions={false}
                showCounts={true}
              />
            </div>

            <div className="flex min-w-0 flex-col items-start justify-start gap-2">
              <p className={labelClass}>{SEARCH_TEXTS.themesLabel}</p>
              <Select
                options={filters.themes.options}
                value={filters.themes.selected}
                placeholderNounForms={THEMES_NOUN_FORMS}
                onChange={filters.themes.onChange}
                placeholder={SEARCH_TEXTS.allPlaceholder}
                allOptionLabel={SEARCH_TEXTS.allPlaceholder}
                multiple={true}
                sortOptions={false}
                showCounts={true}
              />
            </div>

            <div className="flex min-w-0 flex-col items-start justify-start gap-2">
              <p className={labelClass}>{SEARCH_TEXTS.rhymesLabel}</p>
              <Select
                options={filters.rhymes.options}
                value={filters.rhymes.selected}
                placeholderNounForms={RHYMES_NOUN_FORMS}
                onChange={filters.rhymes.onChange}
                placeholder={SEARCH_TEXTS.allPlaceholder}
                allOptionLabel={SEARCH_TEXTS.allPlaceholder}
                multiple={true}
                sortOptions={false}
                showCounts={true}
              />
            </div>

            <div className="flex min-w-0 flex-col items-start justify-start gap-2">
              <p className={labelClass}>{SEARCH_TEXTS.collectionsLabel}</p>
              <Select
                options={filters.collections.options}
                value={filters.collections.selected}
                placeholderNounForms={COLLECTIONS_NOUN_FORMS}
                onChange={filters.collections.onChange}
                placeholder={SEARCH_TEXTS.allPlaceholder}
                allOptionLabel={SEARCH_TEXTS.allPlaceholder}
                multiple={true}
                sortOptions={false}
                showCounts={true}
              />
            </div>
          </>
        )}
      </div>
    </section>
  );
}

type FiltersButtonProps = {
  readonly onToggle: () => void;
  readonly filtersVisible: boolean;
};

export function FiltersButton({ onToggle, filtersVisible }: FiltersButtonProps) {
  return (
    <Button
      type="button"
      tabIndex={0}
      data-filters-toggle
      variant="default"
      onClick={onToggle}
      className={cn(
        'flex h-12 items-center justify-between gap-6 rounded-md border bg-transparent px-4 text-base shadow-none transition-colors outline-none',
        filtersVisible
          ? 'border-text text-text hover:bg-surface-hover'
          : 'border-border text-text-subtle hover:bg-surface-hover hover:text-text-muted'
      )}
      aria-label={filtersVisible ? 'إخفاء الفلاتر' : 'إظهار الفلاتر'}
    >
      <Filter tabIndex={-1} size={16} className="h-4 w-4" />
      {filtersVisible ? (
        <ChevronUp tabIndex={-1} size={16} className="h-4 w-4" />
      ) : (
        <ChevronDown tabIndex={-1} size={16} className="h-4 w-4" />
      )}
    </Button>
  );
}

type ExactToggleProps = {
  readonly enabled: boolean;
  readonly onToggle: () => void;
};

export function ExactToggle({ enabled, onToggle }: ExactToggleProps) {
  return (
    <Button
      type="button"
      tabIndex={0}
      variant="default"
      onClick={onToggle}
      aria-pressed={enabled}
      className={cn(
        'flex h-12 items-center justify-center gap-2 rounded-md border bg-transparent px-4 text-base shadow-none transition-colors outline-none',
        enabled
          ? 'border-text text-text hover:bg-surface-hover'
          : 'border-border text-text-subtle hover:bg-surface-hover hover:text-text-muted'
      )}
      aria-label={
        enabled ? SEARCH_TEXTS.exactToggleDisableAria : SEARCH_TEXTS.exactToggleEnableAria
      }
    >
      <WholeWord tabIndex={-1} size={16} className="h-4 w-4" />
      <span>{SEARCH_TEXTS.exactLabel}</span>
    </Button>
  );
}

type FilterBadgesProps = {
  readonly selectedErasLength: number;
  readonly selectedMetersLength: number;
  readonly selectedPoemTypesLength: number;
  readonly selectedRhymesLength: number;
  readonly selectedThemesLength: number;
  readonly selectedCollectionsLength: number;
  readonly erasCount: string;
  readonly metersCount: string;
  readonly poemTypesCount: string;
  readonly themesCount: string;
  readonly rhymesCount: string;
  readonly collectionsCount: string;
};

export function FilterBadges({
  selectedErasLength,
  selectedMetersLength,
  selectedPoemTypesLength,
  selectedRhymesLength,
  selectedThemesLength,
  selectedCollectionsLength,
  erasCount,
  metersCount,
  poemTypesCount,
  themesCount,
  rhymesCount,
  collectionsCount,
}: FilterBadgesProps) {
  const badgeClassname = 'text-xs md:text-sm text-text-subtle border-border';
  return (
    <div tabIndex={-1} className="flex flex-wrap justify-end gap-1">
      {selectedErasLength > 0 && (
        <Badge variant="outline" className={badgeClassname}>
          {erasCount}
        </Badge>
      )}
      {selectedMetersLength > 0 && (
        <Badge variant="outline" className={badgeClassname}>
          {metersCount}
        </Badge>
      )}
      {selectedPoemTypesLength > 0 && (
        <Badge variant="outline" className={badgeClassname}>
          {poemTypesCount}
        </Badge>
      )}
      {selectedThemesLength > 0 && (
        <Badge variant="outline" className={badgeClassname}>
          {themesCount}
        </Badge>
      )}
      {selectedRhymesLength > 0 && (
        <Badge variant="outline" className={badgeClassname}>
          {rhymesCount}
        </Badge>
      )}
      {selectedCollectionsLength > 0 && (
        <Badge variant="outline" className={badgeClassname}>
          {collectionsCount}
        </Badge>
      )}
    </div>
  );
}
