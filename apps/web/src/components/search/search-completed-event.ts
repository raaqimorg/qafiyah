import { useEffect, useRef } from 'react';

import { captureEvent } from '@/lib/analytics/capture-event';

import type { Properties } from 'posthog-js';

export type SearchSummary = {
  readonly query: string;
  readonly exact: boolean;
  readonly eras: readonly string[];
  readonly meters: readonly string[];
  readonly rhymes: readonly string[];
  readonly themes: readonly string[];
  readonly poemTypes: readonly string[];
  readonly collections: readonly string[];
  readonly poemResults: number;
  readonly poetResults: number;
};

type SearchCompletedEvent = {
  readonly key: string;
  readonly properties: Properties;
};

function searchKey(summary: SearchSummary): string {
  return JSON.stringify([
    summary.query.trim(),
    summary.exact,
    summary.eras,
    summary.meters,
    summary.rhymes,
    summary.themes,
    summary.poemTypes,
    summary.collections,
  ]);
}

function searchCompletedProperties(summary: SearchSummary): Properties {
  const hasResults = summary.poemResults + summary.poetResults > 0;
  const query = summary.query.trim();
  return {
    poem_results: summary.poemResults,
    poet_results: summary.poetResults,
    has_results: hasResults,
    exact: summary.exact,
    era_filters: summary.eras.length,
    meter_filters: summary.meters.length,
    rhyme_filters: summary.rhymes.length,
    theme_filters: summary.themes.length,
    poem_type_filters: summary.poemTypes.length,
    collection_filters: summary.collections.length,
    ...(hasResults || query === '' ? {} : { query }),
  };
}

export function searchCompletedEvent(
  lastKey: string | null,
  isSettled: boolean,
  summary: SearchSummary
): SearchCompletedEvent | null {
  if (!isSettled) return null;
  const key = searchKey(summary);
  if (key === lastKey) return null;
  return { key, properties: searchCompletedProperties(summary) };
}

export function useSearchCompletedEvent(isSettled: boolean, summary: SearchSummary): void {
  const lastKeyRef = useRef<string | null>(null);

  useEffect(() => {
    const event = searchCompletedEvent(lastKeyRef.current, isSettled, summary);
    if (event === null) return;
    lastKeyRef.current = event.key;
    captureEvent('search_completed', event.properties);
  }, [isSettled, summary]);
}
