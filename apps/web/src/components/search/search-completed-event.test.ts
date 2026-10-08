import { describe, expect, it } from 'vitest';

import { searchCompletedEvent } from './search-completed-event';

import type { SearchSummary } from './search-completed-event';

const SEARCH: SearchSummary = {
  query: 'ليلى',
  exact: false,
  eras: ['abbasi'],
  meters: [],
  rhymes: [],
  themes: ['ghazal', 'ritha'],
  poemTypes: [],
  collections: [],
  poemResults: 12,
  poetResults: 1,
};

describe('searchCompletedEvent', () => {
  it('sends nothing while the search is still loading', () => {
    expect(searchCompletedEvent(null, false, SEARCH)).toBeNull();
  });

  it('counts the results and each filter kind', () => {
    expect(searchCompletedEvent(null, true, SEARCH)?.properties).toEqual({
      poem_results: 12,
      poet_results: 1,
      has_results: true,
      exact: false,
      era_filters: 1,
      meter_filters: 0,
      rhyme_filters: 0,
      theme_filters: 2,
      poem_type_filters: 0,
      collection_filters: 0,
    });
  });

  it('keeps the search text only when the search finds nothing', () => {
    const empty = { ...SEARCH, poemResults: 0, poetResults: 0 };
    expect(searchCompletedEvent(null, true, empty)?.properties).toMatchObject({
      has_results: false,
      query: 'ليلى',
    });
    expect(searchCompletedEvent(null, true, SEARCH)?.properties).not.toHaveProperty('query');
  });

  it('sends one event for a search, however often its results change', () => {
    const first = searchCompletedEvent(null, true, SEARCH);
    const morePages = { ...SEARCH, poemResults: 40 };
    expect(searchCompletedEvent(first?.key ?? null, true, morePages)).toBeNull();
  });

  it('sends a new event when the search changes', () => {
    const first = searchCompletedEvent(null, true, SEARCH);
    const exact = { ...SEARCH, exact: true };
    expect(searchCompletedEvent(first?.key ?? null, true, exact)).not.toBeNull();
  });
});
