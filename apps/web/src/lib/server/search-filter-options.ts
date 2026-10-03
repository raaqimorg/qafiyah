import { ResultAsync } from 'neverthrow';

import {
  type SearchFilterOptions,
  sortMeterOptions,
  toSelectOptions,
} from '@/lib/constants/taxonomy-data';
import { allCollections } from '@/lib/server/collections';
import { allEras, allMeters, allPoemTypes, allRhymes, allThemes } from '@/lib/server/taxonomies';

export const NO_SEARCH_FILTER_OPTIONS: SearchFilterOptions = {
  eras: [],
  meters: [],
  rhymes: [],
  themes: [],
  poemTypes: [],
  collections: [],
};

export function loadSearchFilterOptions(): ResultAsync<SearchFilterOptions, unknown> {
  return ResultAsync.fromPromise(
    Promise.all([
      allEras(),
      allMeters(),
      allRhymes(),
      allThemes(),
      allPoemTypes(),
      allCollections(),
    ]),
    (error) => error
  ).map(([eras, meters, rhymes, themes, poemTypes, collections]) => ({
    eras: toSelectOptions(eras),
    meters: sortMeterOptions(toSelectOptions(meters)),
    rhymes: toSelectOptions(rhymes),
    themes: toSelectOptions(themes),
    poemTypes: toSelectOptions(poemTypes),
    collections: toSelectOptions(collections),
  }));
}
