import { FIXTURE_POET } from '../fixtures';
import { WEB } from '../target';

export type SharedEntry = { readonly note: string; readonly url: string };

export type Burst = { readonly note: string; readonly url: string; readonly count: number };

export const sharedEntries: readonly SharedEntry[] = [
  {
    note: 'a poem page with a stray query string is served from the cached page',
    url: `${WEB}/poems/${FIXTURE_POET.poems[0]}`,
  },
  {
    note: 'the homepage with a search query string is served from the cached page',
    url: `${WEB}/`,
  },
];

export const repeatedEntries: readonly SharedEntry[] = [
  {
    note: 'a repeated website search is answered from the nginx cache',
    url: `${WEB}/api/v1/search?q=${encodeURIComponent('قفا نبك')}&types=poems`,
  },
];

export const searchBursts: readonly Burst[] = [
  {
    note: 'rapid poet searches from one visitor are limited with Retry-After, never 5xx',
    url: `${WEB}/poets?q=${encodeURIComponent('حب')}`,
    count: 45,
  },
];
