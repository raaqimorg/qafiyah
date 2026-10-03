import { QueryClientProvider, useQuery } from '@tanstack/react-query';
import { useQueryStates } from 'nuqs';
import { NuqsAdapter } from 'nuqs/adapters/react';
import { useState } from 'react';

import { CardList } from '@/components/card-list';
import { IslandErrorBoundary } from '@/components/island-error-boundary';
import { Pagination } from '@/components/pagination';
import { PoetPoemFilters } from '@/components/poet-poem-filters';
import { ErrorState } from '@/components/ui-extended/error-state';
import { isPlainClick, moveToPageTop, usePageviews } from '@/components/use-list-navigation';
import { apiBrowser } from '@/lib/api/browser-client';
import { formatArabicCount } from '@/lib/arabic';
import { TYPE } from '@/lib/constants/design-tokens';
import { POEMS_NOUN_FORMS } from '@/lib/constants/taxonomy-data';
import { createListQueryClient } from '@/lib/list-query-client';
import { derivePagination } from '@/lib/pagination';
import { canonicalPoetFilters, hasPoetFilters, type PoetFilterSelection } from '@/lib/poet-filters';
import {
  canonicalPoetSearch,
  loadPoetSearchParams,
  poetSearchParsers,
  type PoetSearchValues,
  serializePoetSearch,
} from '@/lib/poet-search-params';
import { poetPoemItems } from '@/lib/seo/poets-page';
import { poetUrl } from '@/lib/urls';

import type { PoemsPage, PoetFacets } from '@/lib/api/result-types';
import type { QueryClient } from '@tanstack/react-query';
import type React from 'react';

type PoetListState = {
  readonly filters: PoetFilterSelection;
  readonly page: number;
};

function toListState({ meter, rhyme, theme, page }: PoetSearchValues): PoetListState {
  return { filters: { meter, rhyme, theme }, page };
}

type Props = {
  readonly poetSlug: string;
  readonly serverSearch: string;
  readonly poems: PoemsPage;
  readonly facets: PoetFacets | undefined;
};

const poemsKey = (slug: string, { filters, page }: PoetListState) =>
  ['poet-poems', slug, filters, page] as const;
const facetsKey = (slug: string, { filters }: PoetListState) =>
  ['poet-facets', slug, filters] as const;

async function fetchPoems(slug: string, { filters, page }: PoetListState): Promise<PoemsPage> {
  const { data, response } = await apiBrowser.GET('/poems', {
    params: {
      query: {
        poet: [slug],
        meter: [...filters.meter],
        rhyme: [...filters.rhyme],
        theme: [...filters.theme],
        page: String(page),
      },
    },
  });
  if (data === undefined) throw new Error(`poems request failed with HTTP ${response.status}`);
  return data;
}

async function fetchFacets(slug: string, { filters }: PoetListState): Promise<PoetFacets> {
  const { data, response } = await apiBrowser.GET('/poems/facets', {
    params: {
      query: {
        poet: slug,
        meter: [...filters.meter],
        rhyme: [...filters.rhyme],
        theme: [...filters.theme],
      },
    },
  });
  if (data === undefined) throw new Error(`facets request failed with HTTP ${response.status}`);
  return data.data;
}

function canFilter(facets: PoetFacets | undefined, isFiltered: boolean): facets is PoetFacets {
  return (
    facets !== undefined &&
    (isFiltered ||
      [facets.meters, facets.rhymes, facets.themes].some((entries) => entries.length > 1))
  );
}

function PoetPoemsList({ poetSlug, facets: serverFacets }: Props) {
  const [params, setParams] = useQueryStates(poetSearchParsers, { history: 'push' });
  const capturePageview = usePageviews();
  const state = toListState(params);
  const pageUrl = poetUrl(poetSlug, { page: state.page, ...state.filters });

  const show = async (requested: PoetSearchValues): Promise<boolean> => {
    const filters = canonicalPoetFilters(requested);
    const next = {
      ...requested,
      meter: [...filters.meter],
      rhyme: [...filters.rhyme],
      theme: [...filters.theme],
    };
    if (serializePoetSearch(next) === serializePoetSearch(params)) return false;
    await setParams(next);
    capturePageview();
    return true;
  };

  const followLink = async (event: React.MouseEvent<HTMLAnchorElement>): Promise<boolean> => {
    if (!isPlainClick(event)) return false;
    event.preventDefault();
    return await show(loadPoetSearchParams(new URL(event.currentTarget.href)));
  };

  const handleLinkClick = (event: React.MouseEvent<HTMLAnchorElement>) => {
    void followLink(event);
  };

  const followPageLink = async (event: React.MouseEvent<HTMLAnchorElement>) => {
    if (await followLink(event)) moveToPageTop();
  };

  const handlePageLinkClick = (event: React.MouseEvent<HTMLAnchorElement>) => {
    void followPageLink(event);
  };

  const poemsQuery = useQuery({
    queryKey: poemsKey(poetSlug, state),
    queryFn: () => fetchPoems(poetSlug, state),
    meta: { pageUrl },
  });
  const facetsQuery = useQuery({
    queryKey: facetsKey(poetSlug, state),
    queryFn: () => fetchFacets(poetSlug, state),
    meta: { pageUrl },
    enabled: serverFacets !== undefined,
  });

  const poems = poemsQuery.data;
  if (poems === undefined) return null;

  const isFiltered = hasPoetFilters(state.filters);
  const facets = facetsQuery.data;
  const isBusy = poemsQuery.isPlaceholderData || facetsQuery.isPlaceholderData;
  const pag = derivePagination(poems.pagination, (page) =>
    poetUrl(poetSlug, { page, ...state.filters })
  );

  const countText = formatArabicCount({
    count: poems.pagination.totalItems,
    nounForms: POEMS_NOUN_FORMS,
  });
  const emptyText = isFiltered ? 'لا توجد قصائد مطابقة.' : 'لا توجد قصائد لهذا الشاعر.';

  const handleApply = (next: PoetFilterSelection) => {
    void show({ meter: [...next.meter], rhyme: [...next.rhyme], theme: [...next.theme], page: 1 });
  };

  return (
    <>
      {canFilter(facets, isFiltered) && (
        <div className="flex w-full flex-col gap-3">
          <PoetPoemFilters
            facets={facets}
            selected={state.filters}
            clearHref={poetUrl(poetSlug)}
            onApply={handleApply}
            onClearClick={handleLinkClick}
          />
          {isFiltered && poems.pagination.totalItems > 0 && (
            <p className={`${TYPE.caption} text-text-subtle`}>{countText}</p>
          )}
        </div>
      )}
      <p role="status" className="sr-only">
        {poems.pagination.totalItems > 0 ? countText : emptyText}
      </p>
      <CardList items={poetPoemItems(poems.data)} emptyText={emptyText} isBusy={isBusy} />
      <Pagination {...pag} onLinkClick={handlePageLinkClick} />
    </>
  );
}

function seededClient({ poetSlug, serverSearch, poems, facets }: Props): QueryClient {
  const client = createListQueryClient((url) => window.location.assign(url));
  const state = toListState(loadPoetSearchParams(serverSearch));
  client.setQueryData(poemsKey(poetSlug, state), poems);
  if (facets !== undefined) client.setQueryData(facetsKey(poetSlug, state), facets);
  return client;
}

export function PoetPoems(props: Props) {
  const [queryClient] = useState(() => seededClient(props));
  return (
    <IslandErrorBoundary
      feature="poet-poems"
      fallback={<ErrorState onRetry={() => window.location.reload()} />}
    >
      <QueryClientProvider client={queryClient}>
        <NuqsAdapter serverSearch={props.serverSearch} processUrlSearchParams={canonicalPoetSearch}>
          <PoetPoemsList {...props} />
        </NuqsAdapter>
      </QueryClientProvider>
    </IslandErrorBoundary>
  );
}
