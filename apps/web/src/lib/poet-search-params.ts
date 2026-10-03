import {
  createLoader,
  createSerializer,
  type inferParserType,
  parseAsInteger,
  parseAsNativeArrayOf,
  parseAsString,
} from 'nuqs';

const parseAsSlugs = parseAsNativeArrayOf(parseAsString);

export const poetSearchParsers = {
  meter: parseAsSlugs,
  rhyme: parseAsSlugs,
  theme: parseAsSlugs,
  page: parseAsInteger.withDefault(1),
};

export type PoetSearchValues = inferParserType<typeof poetSearchParsers>;

export const loadPoetSearchParams = createLoader(poetSearchParsers);

export const serializePoetSearch = createSerializer(poetSearchParsers);

export function canonicalPoetSearch(search: URLSearchParams): URLSearchParams {
  return new URLSearchParams(serializePoetSearch(loadPoetSearchParams(search)));
}
