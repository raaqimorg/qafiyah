import type { ArabicNounForms } from '@/lib/arabic';

export type SelectOption = {
  readonly value: string;
  readonly label: string;
  readonly poemsCount?: number;
};

type TermRow = { readonly name: string; readonly slug: string; readonly poemsCount: number };

export const toSelectOptions = (rows: readonly TermRow[]): readonly SelectOption[] =>
  rows.map((row) => ({ value: row.slug, label: row.name, poemsCount: row.poemsCount }));

export type SearchFilterOptions = {
  readonly eras: readonly SelectOption[];
  readonly meters: readonly SelectOption[];
  readonly rhymes: readonly SelectOption[];
  readonly themes: readonly SelectOption[];
  readonly poemTypes: readonly SelectOption[];
  readonly collections: readonly SelectOption[];
};

export const ERAS_NOUN_FORMS = {
  singular: 'عصر',
  accusative: 'عصرًا',
  dual: 'عصران',
  plural: 'عصور',
} as const satisfies ArabicNounForms;

export const POEM_TYPES_NOUN_FORMS = {
  singular: 'نوع',
  accusative: 'نوعًا',
  dual: 'نوعان',
  plural: 'أنواع',
} as const satisfies ArabicNounForms;

export const METERS_NOUN_FORMS = {
  singular: 'بحر',
  accusative: 'بحرًا',
  dual: 'بحران',
  plural: 'بحور',
} as const satisfies ArabicNounForms;

const STANDARD_METERS_ORDER: readonly string[] = [
  'altawil',
  'almadid',
  'albasit',
  'alwafir',
  'alkamil',
  'alhazaj',
  'alrajz',
  'alramal',
  'alsarie',
  'almunsarih',
  'alkhafif',
  'almudare',
  'almuqtadab',
  'almujtath',
  'almutakarib',
  'almutadarak',
  'alkhabab',
];

const meterRank = (slug: string): number => STANDARD_METERS_ORDER.indexOf(slug);

export function sortMeterOptions(options: readonly SelectOption[]): readonly SelectOption[] {
  const standard = options
    .filter((option) => meterRank(option.value) !== -1)
    .sort((a, b) => meterRank(a.value) - meterRank(b.value));
  const rest = options
    .filter((option) => meterRank(option.value) === -1)
    .sort((a, b) => (b.poemsCount ?? 0) - (a.poemsCount ?? 0));
  return [...standard, ...rest];
}

export const THEMES_NOUN_FORMS = {
  singular: 'غرض',
  accusative: 'غرضًا',
  dual: 'غرضان',
  plural: 'أغراض',
} as const satisfies ArabicNounForms;

export const RHYMES_NOUN_FORMS = {
  singular: 'قافية',
  accusative: 'قافية',
  dual: 'قافيتان',
  plural: 'قوافي',
} as const satisfies ArabicNounForms;

export const COLLECTIONS_NOUN_FORMS = {
  singular: 'ديوان',
  accusative: 'ديوانًا',
  dual: 'ديوانان',
  plural: 'دواوين',
} as const satisfies ArabicNounForms;

export const POEMS_NOUN_FORMS = {
  singular: 'قصيدة',
  accusative: 'قصيدة',
  dual: 'قصيدتان',
  plural: 'قصائد',
} as const satisfies ArabicNounForms;

export const VERSES_NOUN_FORMS = {
  singular: 'بيت',
  accusative: 'بيتًا',
  dual: 'بيتان',
  plural: 'أبيات',
} as const satisfies ArabicNounForms;

export const POETS_NOUN_FORMS = {
  singular: 'شاعر',
  accusative: 'شاعرًا',
  dual: 'شاعران',
  plural: 'شعراء',
} as const satisfies ArabicNounForms;

export const LETTERS_NOUN_FORMS = {
  singular: 'حرف',
  accusative: 'حرفًا',
  dual: 'حرفان',
  plural: 'حروف',
} as const satisfies ArabicNounForms;

export const REQUESTS_NOUN_FORMS = {
  singular: 'طلب',
  accusative: 'طلبًا',
  dual: 'طلبان',
  plural: 'طلبات',
} as const satisfies ArabicNounForms;

export const RESULTS_NOUN_FORMS = {
  singular: 'نتيجة',
  accusative: 'نتيجة',
  dual: 'نتيجتان',
  plural: 'نتائج',
} as const satisfies ArabicNounForms;

export const CLASSICAL_POEM_TYPE = 'amudi';
