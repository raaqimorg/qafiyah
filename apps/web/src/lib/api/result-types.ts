import type { apiBrowser } from './browser-client';
import type { ClientPathsWithMethod, MethodResponse } from 'openapi-fetch';

type Ok<P extends ClientPathsWithMethod<typeof apiBrowser, 'get'>> = MethodResponse<
  typeof apiBrowser,
  'get',
  P
>;

export type Poem = Ok<'/poems/{slug}'>['data'];
export type SearchResponse = Ok<'/search'>;
export type PoemSearchResult = NonNullable<SearchResponse['poems']>['data'][number];
export type PoetSearchResult = NonNullable<SearchResponse['poets']>['data'][number];
export type PoemsPage = Ok<'/poems'>;
export type PoetFacets = Ok<'/poems/facets'>['data'];
