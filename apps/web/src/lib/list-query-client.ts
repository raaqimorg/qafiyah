import { keepPreviousData, QueryCache, QueryClient } from '@tanstack/react-query';

import { REACT_QUERY_GC_TIME_MS, REACT_QUERY_STALE_TIME_MS } from '@/lib/constants/config';

export function createListQueryClient(loadPage: (url: string) => void): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({
      onError: (_error, query) => {
        const pageUrl = query.meta?.['pageUrl'];
        if (typeof pageUrl === 'string') loadPage(pageUrl);
      },
    }),
    defaultOptions: {
      queries: {
        staleTime: REACT_QUERY_STALE_TIME_MS,
        gcTime: REACT_QUERY_GC_TIME_MS,
        retry: false,
        refetchOnWindowFocus: false,
        refetchOnReconnect: false,
        placeholderData: keepPreviousData,
      },
    },
  });
}
