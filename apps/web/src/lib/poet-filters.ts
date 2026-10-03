const POET_FILTER_KEYS = ['meter', 'rhyme', 'theme'] as const;

export type PoetFilterKey = (typeof POET_FILTER_KEYS)[number];

export type PoetFilterSelection = Readonly<Record<PoetFilterKey, readonly string[]>>;

export const NO_POET_FILTERS: PoetFilterSelection = { meter: [], rhyme: [], theme: [] };

type Listed = { readonly slug: string };

type PoetFacetLists = {
  readonly meters: readonly Listed[];
  readonly rhymes: readonly Listed[];
  readonly themes: readonly Listed[];
};

export const POET_FACET_LIST = {
  meter: 'meters',
  rhyme: 'rhymes',
  theme: 'themes',
} as const satisfies Record<PoetFilterKey, keyof PoetFacetLists>;

export function hasPoetFilters(selection: PoetFilterSelection): boolean {
  return POET_FILTER_KEYS.some((key) => selection[key].length > 0);
}

export function everyPoetFilterListed(
  selection: PoetFilterSelection,
  facets: PoetFacetLists
): boolean {
  return POET_FILTER_KEYS.every((key) =>
    selection[key].every((slug) =>
      facets[POET_FACET_LIST[key]].some((entry) => entry.slug === slug)
    )
  );
}

export function samePoetFilters(a: PoetFilterSelection, b: PoetFilterSelection): boolean {
  return POET_FILTER_KEYS.every(
    (key) => a[key].length === b[key].length && a[key].every((slug) => b[key].includes(slug))
  );
}
