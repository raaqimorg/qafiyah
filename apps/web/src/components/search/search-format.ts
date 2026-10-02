import { type ArabicNounForms, formatArabicCount, NON_ARABIC_BASIC_REGEX } from '@/lib/arabic';
import { SEARCH_TEXTS } from '@/lib/constants/copy';
import { RESULTS_NOUN_FORMS } from '@/lib/constants/taxonomy-data';
import { ES_MAX_RESULT_WINDOW } from '@qafiyah/config';

const QUERY_DISPLAY_TRUNCATE_LENGTH = 20;

export function getBadgeCount(count: number, nounForms: ArabicNounForms): string {
  return formatArabicCount({ count, nounForms });
}

export function getNoResultsText({
  hasCommittedQuery,
  query,
}: {
  readonly hasCommittedQuery: boolean;
  readonly query: string;
}): string {
  if (!hasCommittedQuery) return SEARCH_TEXTS.noFilterResultsText;
  const cleaned = query.replace(NON_ARABIC_BASIC_REGEX, '').slice(0, QUERY_DISPLAY_TRUNCATE_LENGTH);
  return `لم يُعثر على نتيجة لـ "${cleaned}${query.length > QUERY_DISPLAY_TRUNCATE_LENGTH ? '...' : ''}"`;
}

export function getSectionResultText({ count }: { readonly count: number }): string {
  const found = formatArabicCount({ count, nounForms: RESULTS_NOUN_FORMS });
  return count >= ES_MAX_RESULT_WINDOW ? `عثر على أكثر من ${found}` : `عثر على ${found}`;
}
