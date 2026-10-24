import type { SearchType } from '@qafiyah/config';

export type SearchQueryInput = {
  readonly q: string;
  readonly type: SearchType;
  readonly cursor?: string | undefined;
  readonly eras: readonly string[];
  readonly meters: readonly string[];
  readonly rhymes: readonly string[];
  readonly themes: readonly string[];
  readonly poemTypes: readonly string[];
  readonly collections: readonly string[];
  readonly exact: boolean;
};

export type SearchQueryParams = {
  readonly q: string;
  readonly types: SearchType[];
  readonly poemsCursor?: string;
  readonly poetsCursor?: string;
  readonly eraSlugs: string[];
  readonly meterSlugs: string[];
  readonly rhymeSlugs: string[];
  readonly themeSlugs: string[];
  readonly poemTypeSlugs: string[];
  readonly collectionSlugs: string[];
  readonly poetSlugs: string[];
  readonly exact: 'true' | 'false';
};

export function searchQueryParams(input: SearchQueryInput): SearchQueryParams {
  const poems = input.type === 'poems';
  const { cursor } = input;
  return {
    q: input.q,
    types: [input.type],
    ...(poems && cursor !== undefined ? { poemsCursor: cursor } : {}),
    ...(!poems && cursor !== undefined ? { poetsCursor: cursor } : {}),
    eraSlugs: [...input.eras],
    meterSlugs: poems ? [...input.meters] : [],
    rhymeSlugs: poems ? [...input.rhymes] : [],
    themeSlugs: poems ? [...input.themes] : [],
    poemTypeSlugs: poems ? [...input.poemTypes] : [],
    collectionSlugs: poems ? [...input.collections] : [],
    poetSlugs: [],
    exact: input.exact ? 'true' : 'false',
  };
}

export function searchQueryKey(input: SearchQueryInput): readonly unknown[] {
  const { cursor: _cursor, ...rest } = input;
  return ['search', input.type, rest];
}
