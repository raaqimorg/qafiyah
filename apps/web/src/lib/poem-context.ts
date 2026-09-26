import type { Poem } from '@/lib/api/result-types';
import type { TaxonomySection } from '@/lib/urls';

export type PoemNavScope = 'meter' | 'rhyme' | 'theme' | 'collection';

type ContextTerm = { readonly name: string; readonly slug: string };

type PoemContext = {
  readonly by: PoemNavScope;
  readonly crumbLabel: string;
  readonly navLabel: string;
  readonly term: (poem: Poem) => ContextTerm | undefined;
};

export const POET_NAV_LABEL = 'تصفح قصائد الشاعر';

export const POEM_CONTEXTS = {
  meters: {
    by: 'meter',
    crumbLabel: 'البحور',
    navLabel: 'تصفح قصائد البحر',
    term: (poem) => poem.meter,
  },
  rhymes: {
    by: 'rhyme',
    crumbLabel: 'القوافي',
    navLabel: 'تصفح قصائد القافية',
    term: (poem) => poem.rhyme,
  },
  themes: {
    by: 'theme',
    crumbLabel: 'الأغراض',
    navLabel: 'تصفح قصائد الغرض',
    term: (poem) => poem.theme,
  },
  collections: {
    by: 'collection',
    crumbLabel: 'الدواوين',
    navLabel: 'تصفح قصائد الديوان',
    term: (poem) => poem.collection,
  },
} as const satisfies Record<TaxonomySection, PoemContext>;

const SECTIONS: readonly TaxonomySection[] = ['meters', 'rhymes', 'themes', 'collections'];

export function parsePoemContext(value: string | null): TaxonomySection | undefined {
  return SECTIONS.find((section) => section === value);
}

export function resolvePoemContext(
  poem: Poem,
  from: TaxonomySection | undefined
): TaxonomySection | undefined {
  if (from === undefined) return undefined;
  return POEM_CONTEXTS[from].term(poem) === undefined ? undefined : from;
}
