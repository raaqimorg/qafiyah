import { toArabicDigits } from '@/lib/arabic';
import { SITE_NAME_AR } from '@/lib/constants/site-meta';

const META_DESCRIPTION_OPTIMAL_LENGTH = 300;
const TITLE_DISPLAY_LENGTH = 60;
export const UNKNOWN_ENTITY_NAME = 'غير معروف';

export function sanitizeMetaText(value: string): string {
  return value
    .replaceAll(/['"\\]/g, '')
    .replaceAll(/\s+/g, ' ')
    .trim();
}

export function truncateMetaText(value: string): string {
  return value.length > META_DESCRIPTION_OPTIMAL_LENGTH
    ? value.slice(0, META_DESCRIPTION_OPTIMAL_LENGTH)
    : value;
}

export function excerptAtWordBoundary(value: string, maxLength: number): string {
  if (value.length <= maxLength) return value;
  const cut = value.slice(0, maxLength);
  const lastSpace = cut.lastIndexOf(' ');
  return lastSpace > 0 ? cut.slice(0, lastSpace) : cut;
}

export function withBrand(lead: string): string {
  return `${lead} | ${SITE_NAME_AR}`;
}

export function withBrandIfItFits(lead: string): string {
  const branded = withBrand(lead);
  return branded.length <= TITLE_DISPLAY_LENGTH ? branded : lead;
}

export function withPageNumber(lead: string, page: number): string {
  return page > 1 ? `${lead}، الصفحة ${toArabicDigits(page)}` : lead;
}
