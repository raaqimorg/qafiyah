import { POEM_CONTEXTS, POET_NAV_LABEL } from '@/lib/poem-context';
import { poemInSectionUrl, type TaxonomySection } from '@/lib/urls';

type AdjacentPoemRef = {
  readonly title: string;
  readonly slug: string;
};

type AdjacentPoemsSource = {
  readonly prev?: AdjacentPoemRef | undefined;
  readonly next?: AdjacentPoemRef | undefined;
};

export type AdjacentPoemsView = {
  readonly label: string;
  readonly prevHref: string | undefined;
  readonly prevTitle: string | undefined;
  readonly nextHref: string | undefined;
  readonly nextTitle: string | undefined;
  readonly hidden: boolean;
};

export function deriveAdjacentPoems(
  poem: AdjacentPoemsSource,
  context?: TaxonomySection
): AdjacentPoemsView {
  return {
    label: context === undefined ? POET_NAV_LABEL : POEM_CONTEXTS[context].navLabel,
    prevHref: poem.prev ? poemInSectionUrl(poem.prev.slug, context) : undefined,
    prevTitle: poem.prev?.title,
    nextHref: poem.next ? poemInSectionUrl(poem.next.slug, context) : undefined,
    nextTitle: poem.next?.title,
    hidden: poem.prev === undefined && poem.next === undefined,
  };
}
