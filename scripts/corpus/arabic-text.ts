export type SoundClassTable = Readonly<Record<string, string>>;

const DIACRITICS_REGEX = /[ً-ٰٟۖ-ۭـ]/g;
const NON_LETTER_REGEX = /[^ء-غف-ي]/g;

export function stripDiacritics(text: string): string {
  return text.replaceAll(DIACRITICS_REGEX, '');
}

export function lettersOnly(text: string): string {
  return stripDiacritics(text).replaceAll(NON_LETTER_REGEX, '');
}

export const DEFAULT_SOUND_CLASSES: SoundClassTable = {
  ء: 'ء',
  ؤ: 'ء',
  ئ: 'ء',
  ا: 'ا',
  ى: 'ا',
  ة: 'ة',
  ه: 'ة',
  ت: 'ة',
};

export const OPTIONAL_SOUND_CLASSES: SoundClassTable = {
  ض: 'ض',
  ظ: 'ض',
};

export function normalizeLetter(letter: string, table: SoundClassTable): string {
  return table[letter] ?? letter;
}
