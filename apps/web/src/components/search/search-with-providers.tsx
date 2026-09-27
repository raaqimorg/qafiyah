'use client';

import { QueryCache, QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { NuqsAdapter } from 'nuqs/adapters/react';
import { useEffect, useState } from 'react';

import { IslandErrorBoundary } from '@/components/island-error-boundary';
import { ErrorState } from '@/components/ui-extended/error-state';
import {
  REACT_QUERY_GC_TIME_MS,
  REACT_QUERY_RETRY_COUNT,
  REACT_QUERY_STALE_TIME_MS,
  SEARCH_NETWORK_RETRY_COUNT,
  SEARCH_RETRY_BASE_DELAY_MS,
  SEARCH_RETRY_MAX_DELAY_MS,
} from '@/lib/constants/config';
import { isTransientNetworkError } from '@/lib/observability/is-transient-network-error';
import { reportError } from '@/lib/observability/report-error';

import { SearchContainer } from './search-container';

import type { SearchFilterOptions } from '@/lib/constants/taxonomy-data';

export function SearchWithProviders({
  filterOptions,
}: {
  readonly filterOptions: SearchFilterOptions;
}) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        queryCache: new QueryCache({
          onError: (error, query) => {
            if (isTransientNetworkError(error)) {
              return;
            }
            reportError('search query failed', error, {
              feature: 'search',
              extra: { queryKey: query.queryKey },
            });
          },
        }),
        defaultOptions: {
          queries: {
            staleTime: REACT_QUERY_STALE_TIME_MS,
            gcTime: REACT_QUERY_GC_TIME_MS,
            refetchOnWindowFocus: false,
            refetchOnMount: false,
            refetchOnReconnect: false,
            retry: (failureCount, error) =>
              failureCount <=
              (isTransientNetworkError(error)
                ? SEARCH_NETWORK_RETRY_COUNT
                : REACT_QUERY_RETRY_COUNT),
            retryDelay: (attemptIndex) =>
              Math.min(SEARCH_RETRY_BASE_DELAY_MS * 2 ** attemptIndex, SEARCH_RETRY_MAX_DELAY_MS),
          },
        },
      })
  );

  useEffect(() => {
    document.querySelector('#search-shell')?.remove();
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <IslandErrorBoundary
        feature="search"
        fallback={<ErrorState onRetry={() => window.location.reload()} />}
      >
        <NuqsAdapter>
          <SearchContainer filterOptions={filterOptions} />
        </NuqsAdapter>
      </IslandErrorBoundary>
    </QueryClientProvider>
  );
}
