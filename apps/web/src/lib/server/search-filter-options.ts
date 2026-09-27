import { ResultAsync } from 'neverthrow';

import {
  type SearchFilterOptions,
  type SelectOption,
  sortMeterOptions,
} from '@/lib/constants/taxonomy-data';
import { allCollections } from '@/lib/server/collections';
import { allEras, allMeters, allRhymes, allThemes } from '@/lib/server/taxonomies';

type TermRow = { readonly name: string; readonly slug: string; readonly poemsCount: number };

const toOptions = (rows: readonly TermRow[]): readonly SelectOption[] =>
  rows.map((row) => ({ value: row.slug, label: row.name, poemsCount: row.poemsCount }));

export const NO_SEARCH_FILTER_OPTIONS: SearchFilterOptions = {
  eras: [],
  meters: [],
  rhymes: [],
  themes: [],
  collections: [],
};

export function loadSearchFilterOptions(): ResultAsync<SearchFilterOptions, unknown> {
  return ResultAsync.fromPromise(
    Promise.all([allEras(), allMeters(), allRhymes(), allThemes(), allCollections()]),
    (error) => error
  ).map(([eras, meters, rhymes, themes, collections]) => ({
    eras: toOptions(eras),
    meters: sortMeterOptions(toOptions(meters)),
    rhymes: toOptions(rhymes),
    themes: toOptions(themes),
    collections: toOptions(collections),
  }));
}
