'use client';

import { useInfiniteQuery } from '@tanstack/react-query';
import { parseAsBoolean, useQueryState } from 'nuqs';
import { useEffect, useState } from 'react';

import { apiBrowser } from '@/lib/api/browser-client';
import { searchQueryKey, searchQueryParams } from '@/lib/api/search-query';
import { sanitizeArabicInput, stripInputNoise } from '@/lib/arabic';
import { SEARCH_TEXTS } from '@/lib/constants/copy';
import { DEFAULT_ERA_SLUGS } from '@/lib/constants/taxonomy-data';
import { createCsvFilterSetter, splitCsvIds, validateText } from '@/lib/search/csv-filters';

import { deriveSearchStatus, deriveSectionStatus } from './search-status';

import type { PoemSearchResult, PoetSearchResult, SearchResponse } from '@/lib/api/result-types';
import type { SearchQueryInput } from '@/lib/api/search-query';
import type React from 'react';

const INFINITE_SCROLL_THRESHOLD = 0.1;
const SEARCH_RESULTS_STALE_TIME_MS = 5 * 60 * 1000;

async function fetchSearch(input: SearchQueryInput): Promise<SearchResponse> {
  const { data, error, response } = await apiBrowser.GET('/search', {
    params: { query: searchQueryParams(input) },
  });
  if (data === undefined) {
    throw error instanceof Error
      ? error
      : new Error(`search request failed with HTTP ${response.status}`);
  }
  return data;
}

function useInfiniteScroll(
  fetchNextPage: () => unknown,
  hasNextPage: boolean | undefined,
  isFetchingNextPage: boolean
) {
  const [sentinel, setSentinel] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!sentinel) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting === true && hasNextPage === true && !isFetchingNextPage) {
          void fetchNextPage();
        }
      },
      { root: null, rootMargin: '200px', threshold: INFINITE_SCROLL_THRESHOLD }
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [sentinel, hasNextPage, isFetchingNextPage, fetchNextPage]);

  return { loadMoreRef: setSentinel };
}

type PoemsSectionViewModel = {
  readonly items: readonly PoemSearchResult[];
  readonly total: number;
  readonly isFetchingMore: boolean;
  readonly onLoadMore: () => void;
  readonly hasNextPage: boolean;
};

type PoetsSectionViewModel = {
  readonly items: readonly PoetSearchResult[];
  readonly total: number;
  readonly isFetchingMore: boolean;
  readonly loadMoreRef: React.Ref<HTMLDivElement>;
};

export function useSearch() {
  const [query, setQuery] = useQueryState('q', { defaultValue: '' });
  const [eraIds, setEraIds] = useQueryState('era_slugs', { defaultValue: '' });
  const [meterIds, setMeterIds] = useQueryState('meter_slugs', { defaultValue: '' });
  const [rhymeIds, setRhymeIds] = useQueryState('rhyme_slugs', { defaultValue: '' });
  const [themeIds, setThemeIds] = useQueryState('theme_slugs', { defaultValue: '' });
  const [collectionIds, setCollectionIds] = useQueryState('collection_slugs', {
    defaultValue: '',
  });
  const [exact, setExact] = useQueryState('exact', parseAsBoolean.withDefault(false));
  const [inputValue, setInputValue] = useState(query);
  const [syncedQuery, setSyncedQuery] = useState(query);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [filtersVisible, setFiltersVisible] = useState(false);

  if (query !== syncedQuery) {
    setSyncedQuery(query);
    setInputValue(query);
  }

  const wantPoems = true;
  const wantPoets = true;

  const erasFromUrl = splitCsvIds(eraIds);
  const erasCustomized = erasFromUrl.length > 0;
  const selectedEras = erasCustomized ? erasFromUrl : DEFAULT_ERA_SLUGS;
  const selectedRhymes = splitCsvIds(rhymeIds);
  const selectedMeters = splitCsvIds(meterIds);
  const selectedThemes = splitCsvIds(themeIds);
  const selectedCollections = splitCsvIds(collectionIds);

  const hasFilters =
    erasCustomized ||
    selectedRhymes.length > 0 ||
    selectedMeters.length > 0 ||
    selectedThemes.length > 0 ||
    selectedCollections.length > 0;
  const hasPoetFilters = erasCustomized;
  const hasCommittedQuery = query.trim().length > 0;
  const hasInputText = inputValue.trim().length > 0;
  const canSearchPoems = hasCommittedQuery || hasFilters;
  const canSearchPoets = hasCommittedQuery || hasPoetFilters;

  const searchInput = (type: SearchQueryInput['type'], page: number): SearchQueryInput => ({
    q: query,
    type,
    page,
    eras: type === 'poets' && !erasCustomized ? [] : selectedEras,
    meters: selectedMeters,
    rhymes: selectedRhymes,
    themes: selectedThemes,
    collections: selectedCollections,
    exact,
  });

  const poemsInfiniteQuery = useInfiniteQuery({
    queryKey: searchQueryKey(searchInput('poems', 1)),
    queryFn: ({ pageParam }) => fetchSearch(searchInput('poems', pageParam)),
    initialPageParam: 1,
    getNextPageParam: (lastPage: SearchResponse) => {
      const p = lastPage.poems?.pagination;
      return p && p.page < p.totalPages ? p.page + 1 : undefined;
    },
    enabled: canSearchPoems && wantPoems,
    staleTime: SEARCH_RESULTS_STALE_TIME_MS,
    refetchOnWindowFocus: false,
  });

  const poetsInfiniteQuery = useInfiniteQuery({
    queryKey: searchQueryKey(searchInput('poets', 1)),
    queryFn: ({ pageParam }) => fetchSearch(searchInput('poets', pageParam)),
    initialPageParam: 1,
    getNextPageParam: (lastPage: SearchResponse) => {
      const p = lastPage.poets?.pagination;
      return p && p.page < p.totalPages ? p.page + 1 : undefined;
    },
    enabled: canSearchPoets && wantPoets,
    staleTime: SEARCH_RESULTS_STALE_TIME_MS,
    refetchOnWindowFocus: false,
  });

  const poemItems: readonly PoemSearchResult[] =
    poemsInfiniteQuery.data?.pages.flatMap((p) => p.poems?.data ?? []) ?? [];
  const poemsTotal = poemsInfiniteQuery.data?.pages[0]?.poems?.pagination.totalItems ?? 0;

  const poetItems: readonly PoetSearchResult[] =
    poetsInfiniteQuery.data?.pages.flatMap((p) => p.poets?.data ?? []) ?? [];
  const poetsTotal = poetsInfiniteQuery.data?.pages[0]?.poets?.pagination.totalItems ?? 0;

  const poemsStatus = deriveSectionStatus({
    canSearch: canSearchPoems && wantPoems,
    isError: poemsInfiniteQuery.isError,
    isFetchingNextPage: poemsInfiniteQuery.isFetchingNextPage,
    isSuccess: poemsInfiniteQuery.isSuccess,
    items: poemItems,
  });
  const poetsStatus = deriveSectionStatus({
    canSearch: canSearchPoets && wantPoets,
    isError: poetsInfiniteQuery.isError,
    isFetchingNextPage: poetsInfiniteQuery.isFetchingNextPage,
    isSuccess: poetsInfiniteQuery.isSuccess,
    items: poetItems,
  });

  const searchStatus = deriveSearchStatus([
    { status: poemsStatus, total: poemsTotal },
    { status: poetsStatus, total: poetsTotal },
  ]);

  const { loadMoreRef: poetsLoadMoreRef } = useInfiniteScroll(
    poetsInfiniteQuery.fetchNextPage,
    poetsInfiniteQuery.hasNextPage,
    poetsInfiniteQuery.isFetchingNextPage
  );

  const handleErasChange = createCsvFilterSetter(setEraIds);
  const handleMetersChange = createCsvFilterSetter(setMeterIds);
  const handleRhymesChange = createCsvFilterSetter(setRhymeIds);
  const handleThemesChange = createCsvFilterSetter(setThemeIds);
  const handleCollectionsChange = createCsvFilterSetter(setCollectionIds);

  const handleInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const raw = event.target.value;
    const sanitized = sanitizeArabicInput(raw);
    setInputValue(sanitized);
    if (stripInputNoise(raw) !== sanitized) {
      setValidationError(SEARCH_TEXTS.arabicOnlyError);
      return;
    }
    setValidationError(validateText(sanitized));
  };

  const handleSearch = () => {
    const trimmed = inputValue.trim();
    const error = validateText(trimmed);
    if (error !== null) {
      setValidationError(error);
      return;
    }
    setValidationError(null);
    if (filtersVisible) setFiltersVisible(false);
    if (trimmed !== query.trim()) void setQuery(trimmed, { history: 'push' });
  };

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      handleSearch();
    }
  };

  const handleInputBlur = () => {
    const trimmed = inputValue.trim();
    if (trimmed.length === 0 || trimmed === query.trim()) return;
    handleSearch();
  };

  const handleToggleFilters = () => {
    setFiltersVisible(!filtersVisible);
  };

  const handleToggleExact = () => {
    void setExact((prev) => !prev);
  };

  const handleRetry = () => {
    void poemsInfiniteQuery.refetch();
    void poetsInfiniteQuery.refetch();
  };

  const handleClearInput = () => {
    setInputValue('');
    setValidationError(null);
  };

  const hasQueryToShow = searchStatus.kind !== 'loading' && hasInputText;

  const poems: PoemsSectionViewModel = {
    items: poemItems,
    total: poemsTotal,
    isFetchingMore: poemsInfiniteQuery.isFetchingNextPage,
    onLoadMore: () => {
      void poemsInfiniteQuery.fetchNextPage();
    },
    hasNextPage: poemsInfiniteQuery.hasNextPage,
  };

  const poets: PoetsSectionViewModel = {
    items: poetItems,
    total: poetsTotal,
    isFetchingMore: poetsInfiniteQuery.isFetchingNextPage,
    loadMoreRef: poetsLoadMoreRef,
  };

  return {
    status: searchStatus,
    input: {
      inputValue,
      validationError,
      query,
    },
    sections: {
      poems,
      poets,
    },
    flags: {
      filtersVisible,
      hasQueryToShow,
      hasCommittedQuery,
      hasInputText,
      hasFilters,
      erasCustomized,
      wantPoems,
      wantPoets,
      exactEnabled: exact,
    },
    selection: {
      eras: selectedEras,
      meters: selectedMeters,
      themes: selectedThemes,
      rhymes: selectedRhymes,
      collections: selectedCollections,
    },
    handlers: {
      onInputChange: handleInputChange,
      onKeyDown: handleKeyDown,
      onInputBlur: handleInputBlur,
      onSearch: handleSearch,
      onErasChange: handleErasChange,
      onMetersChange: handleMetersChange,
      onThemesChange: handleThemesChange,
      onRhymesChange: handleRhymesChange,
      onCollectionsChange: handleCollectionsChange,
      onToggleFilters: handleToggleFilters,
      onToggleExact: handleToggleExact,
      onClearInput: handleClearInput,
      onRetry: handleRetry,
    },
  };
}
