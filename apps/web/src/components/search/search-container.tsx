'use client';

import { useEffect, useRef } from 'react';
import { match } from 'ts-pattern';

import {
  getBadgeCount,
  getNoResultsText,
  getSectionResultText,
} from '@/components/search/search-format';
import { useSearch } from '@/components/search/use-search';
import { ErrorState } from '@/components/ui-extended/error-state';
import { SearchInput } from '@/components/ui-extended/search-input';
import { LoadingState, NoResultsState } from '@/components/ui-extended/search-states';
import { Card } from '@/components/ui/card';
import { SEARCH_TEXTS } from '@/lib/constants/copy';
import { TYPE } from '@/lib/constants/design-tokens';
import { SITE_NAME_AR } from '@/lib/constants/site-meta';
import {
  COLLECTIONS_NOUN_FORMS,
  ERAS_NOUN_FORMS,
  METERS_NOUN_FORMS,
  RHYMES_NOUN_FORMS,
  type SearchFilterOptions,
  THEMES_NOUN_FORMS,
} from '@/lib/constants/taxonomy-data';
import { cn } from '@/lib/utils';

import { ExactToggle, FilterBadges, Filters, FiltersButton } from './filters';
import { SearchSection } from './sections';

export function SearchContainer({
  filterOptions,
}: {
  readonly filterOptions: SearchFilterOptions;
}) {
  const { input, status, sections, flags, selection, handlers } = useSearch();

  const poetsHeaderRef = useRef<HTMLHeadingElement>(null);
  const poemsHeaderRef = useRef<HTMLHeadingElement>(null);
  const pendingFocusRef = useRef(false);
  const prevQueryRef = useRef(input.query);

  useEffect(() => {
    if (input.query !== prevQueryRef.current) {
      prevQueryRef.current = input.query;
      pendingFocusRef.current = true;
    }
  }, [input.query]);

  useEffect(() => {
    if (status.kind === 'results' && pendingFocusRef.current) {
      pendingFocusRef.current = false;
      if (sections.poets.items.length > 0) {
        poetsHeaderRef.current?.focus();
      } else {
        poemsHeaderRef.current?.focus();
      }
    }
  }, [status.kind, sections.poets.items.length]);

  const noResultsText = getNoResultsText({
    hasCommittedQuery: flags.hasCommittedQuery,
    query: input.query,
  });

  const poemsResultText = getSectionResultText({ count: sections.poems.total });
  const poetsResultText = getSectionResultText({ count: sections.poets.total });

  const placeholder = flags.wantPoems
    ? SEARCH_TEXTS.poemsSearchPlaceholder
    : SEARCH_TEXTS.poetsSearchPlaceholder;

  return (
    <section className="mx-auto flex w-full max-w-3xl flex-col justify-start pb-24">
      <div
        className={cn(
          'w-full transition-[height] duration-300',
          status.kind === 'idle' ? 'h-[25svh]' : 'h-[8svh]'
        )}
      ></div>

      <div className="flex w-full flex-col gap-4 md:gap-6" dir="rtl">
        <div className="flex flex-col gap-10 md:gap-16">
          <h1 className="sr-only">{SITE_NAME_AR}</h1>
          <p
            className={cn(
              'flex items-center justify-center py-2 text-center',
              TYPE.display,
              'text-text'
            )}
          >
            {SEARCH_TEXTS.currentHeaderTitle}
          </p>
          <Card className="border-0 bg-transparent shadow-none">
            <div className="bg p-0">
              <div className="flex flex-col gap-4">
                <SearchInput
                  placeholder={placeholder}
                  searchLabel={SEARCH_TEXTS.search}
                  inputValue={input.inputValue}
                  validationError={input.validationError}
                  onKeyDown={handlers.onKeyDown}
                  onInputChange={handlers.onInputChange}
                  onInputBlur={handlers.onInputBlur}
                  onClearInput={handlers.onClearInput}
                  onSearch={handlers.onSearch}
                  hasQueryToShow={flags.hasQueryToShow}
                />
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <FiltersButton
                      onToggle={handlers.onToggleFilters}
                      filtersVisible={flags.filtersVisible}
                    />
                    <ExactToggle enabled={flags.exactEnabled} onToggle={handlers.onToggleExact} />
                  </div>

                  <FilterBadges
                    erasCount={getBadgeCount(selection.eras.length || 0, ERAS_NOUN_FORMS)}
                    metersCount={getBadgeCount(selection.meters.length || 0, METERS_NOUN_FORMS)}
                    themesCount={getBadgeCount(selection.themes.length || 0, THEMES_NOUN_FORMS)}
                    rhymesCount={getBadgeCount(selection.rhymes.length || 0, RHYMES_NOUN_FORMS)}
                    collectionsCount={getBadgeCount(
                      selection.collections.length || 0,
                      COLLECTIONS_NOUN_FORMS
                    )}
                    selectedErasLength={selection.eras.length}
                    selectedMetersLength={selection.meters.length}
                    selectedRhymesLength={selection.rhymes.length}
                    selectedThemesLength={selection.themes.length}
                    selectedCollectionsLength={selection.collections.length}
                  />
                </div>

                {flags.filtersVisible && (
                  <Filters
                    filters={{
                      eras: {
                        selected: selection.eras,
                        options: filterOptions.eras,
                        onChange: handlers.onErasChange,
                      },
                      meters: {
                        selected: selection.meters,
                        options: filterOptions.meters,
                        onChange: handlers.onMetersChange,
                      },
                      themes: {
                        selected: selection.themes,
                        options: filterOptions.themes,
                        onChange: handlers.onThemesChange,
                      },
                      rhymes: {
                        selected: selection.rhymes,
                        options: filterOptions.rhymes,
                        onChange: handlers.onRhymesChange,
                      },
                      collections: {
                        selected: selection.collections,
                        options: filterOptions.collections,
                        onChange: handlers.onCollectionsChange,
                      },
                    }}
                    wantPoems={flags.wantPoems}
                  />
                )}
              </div>
            </div>
          </Card>
        </div>

        <div className="flex flex-col gap-10">
          {match(status)
            .with({ kind: 'idle' }, () => null)
            .with({ kind: 'loading' }, () => <LoadingState />)
            .with({ kind: 'error' }, () => <ErrorState onRetry={handlers.onRetry} />)
            .with({ kind: 'empty' }, () => <NoResultsState noResultsText={noResultsText} />)
            .with({ kind: 'results' }, () => (
              <div className="flex flex-col gap-3 px-0.5 py-1">
                {flags.wantPoets && (
                  <SearchSection
                    title={SEARCH_TEXTS.poetsSectionTitle}
                    items={sections.poets.items}
                    resultText={poetsResultText}
                    isFetchingMore={sections.poets.isFetchingMore}
                    loadMoreRef={sections.poets.loadMoreRef}
                    layout="horizontal"
                    headingRef={poetsHeaderRef}
                  />
                )}

                {flags.wantPoems && (
                  <SearchSection
                    title={SEARCH_TEXTS.poemsSectionTitle}
                    items={sections.poems.items}
                    resultText={poemsResultText}
                    isFetchingMore={sections.poems.isFetchingMore}
                    layout="vertical"
                    onLoadMore={sections.poems.onLoadMore}
                    hasNextPage={sections.poems.hasNextPage}
                    headingRef={poemsHeaderRef}
                  />
                )}
              </div>
            ))
            .exhaustive()}
        </div>
      </div>
    </section>
  );
}
