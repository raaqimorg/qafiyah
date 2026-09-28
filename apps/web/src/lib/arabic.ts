import { match } from 'ts-pattern';

export type ArabicNounForms = {
  readonly singular: string;
  readonly dual: string;
  readonly plural: string;
};

const DIGIT_LOOKUP: Readonly<Record<string, string>> = {
  '0': '٠',
  '1': '١',
  '2': '٢',
  '3': '٣',
  '4': '٤',
  '5': '٥',
  '6': '٦',
  '7': '٧',
  '8': '٨',
  '9': '٩',
};

export function toArabicDigits(input: number | string): string {
  return String(input).replaceAll(/[0-9]/g, (d) => DIGIT_LOOKUP[d] ?? d);
}

const ARABIC_NUMBER_FORMAT = new Intl.NumberFormat('ar-SA');

export function formatArabicNumber(value: number): string {
  return ARABIC_NUMBER_FORMAT.format(value);
}

const ARABIC_PLURAL_RULES = new Intl.PluralRules('ar');

export function formatArabicCount({
  count,
  nounForms,
}: {
  readonly count: number;
  readonly nounForms: ArabicNounForms;
}): string {
  const { singular, dual, plural } = nounForms;
  const absoluteCount = Math.abs(count);

  return match(ARABIC_PLURAL_RULES.select(absoluteCount))
    .with('zero', () => `لا ${singular}`)
    .with('one', () => singular)
    .with('two', () => dual)
    .with('few', () => `${formatArabicNumber(absoluteCount)} ${plural}`)
    .with('many', 'other', () => `${formatArabicNumber(absoluteCount)} ${singular}`)
    .exhaustive();
}

const NON_ARABIC_AND_SPACE_REGEX = /[^؀-ۿݐ-ݿࢠ-ࣿ\s]/g;

export const NON_ARABIC_BASIC_REGEX = /[^؀-ۿ\s]/g;

const WHITESPACE_RUN_REGEX = /\s+/g;
const LEADING_WHITESPACE_REGEX = /^\s+/;

const INVISIBLE_FORMATTING_REGEX = /[\u200B-\u200F\u202A-\u202E\u2066-\u2069\u061C]/g;

export function sanitizeArabicInput(raw: string): string {
  return raw
    .replace(INVISIBLE_FORMATTING_REGEX, '')
    .replace(NON_ARABIC_AND_SPACE_REGEX, '')
    .replace(WHITESPACE_RUN_REGEX, ' ')
    .replace(LEADING_WHITESPACE_REGEX, '');
}

export function stripInputNoise(raw: string): string {
  return raw
    .replace(INVISIBLE_FORMATTING_REGEX, '')
    .replace(WHITESPACE_RUN_REGEX, ' ')
    .replace(LEADING_WHITESPACE_REGEX, '');
}

const TASHKEEL_REGEX = /[ؐ-ًؚ-ٰٟۖ-ۭـ]/g;

export function stripTashkeel(text: string): string {
  return text.replace(TASHKEEL_REGEX, '');
}
