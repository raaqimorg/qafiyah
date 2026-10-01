import type { ArabicNounForms } from '@/lib/arabic';

export type SelectOption = {
  readonly value: string;
  readonly label: string;
  readonly poemsCount?: number;
};

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
  dual: 'عصران',
  plural: 'عصور',
} as const satisfies ArabicNounForms;

export const POEM_TYPES_NOUN_FORMS = {
  singular: 'نوع',
  dual: 'نوعان',
  plural: 'أنواع',
} as const satisfies ArabicNounForms;

export const METERS_NOUN_FORMS = {
  singular: 'بحر',
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
  dual: 'غرضان',
  plural: 'أغراض',
} as const satisfies ArabicNounForms;

export const RHYMES_NOUN_FORMS = {
  singular: 'قافية',
  dual: 'قافيتان',
  plural: 'قوافي',
} as const satisfies ArabicNounForms;

export const COLLECTIONS_NOUN_FORMS = {
  singular: 'ديوان',
  dual: 'ديوانان',
  plural: 'دواوين',
} as const satisfies ArabicNounForms;

export const POEMS_NOUN_FORMS = {
  singular: 'قصيدة',
  dual: 'قصيدتان',
  plural: 'قصائد',
} as const satisfies ArabicNounForms;

export const VERSES_NOUN_FORMS = {
  singular: 'بيت',
  dual: 'بيتان',
  plural: 'أبيات',
} as const satisfies ArabicNounForms;

export const POETS_NOUN_FORMS = {
  singular: 'شاعر',
  dual: 'شاعران',
  plural: 'شعراء',
} as const satisfies ArabicNounForms;

export const LETTERS_NOUN_FORMS = {
  singular: 'حرف',
  dual: 'حرفان',
  plural: 'حروف',
} as const satisfies ArabicNounForms;

export const RESULTS_NOUN_FORMS = {
  singular: 'نتيجة',
  dual: 'نتيجتان',
  plural: 'نتائج',
} as const satisfies ArabicNounForms;

export const CLASSICAL_POEM_TYPE = 'amudi';
