import { describe, expect, it, test } from 'vitest';

import { VERSES_NOUN_FORMS } from '@/lib/constants/taxonomy-data';

import {
  type ArabicNounForms,
  formatArabicCount,
  formatArabicNumber,
  NON_ARABIC_BASIC_REGEX,
  sanitizeArabicInput,
  stripInputNoise,
  stripTashkeel,
  toArabicDigits,
} from './arabic';

describe('toArabicDigits', () => {
  it('converts western digits to Arabic-Indic digits', () => {
    expect(toArabicDigits(0)).toBe('٠');
    expect(toArabicDigits(1)).toBe('١');
    expect(toArabicDigits(9)).toBe('٩');
  });

  it('converts a multi-digit number', () => {
    expect(toArabicDigits(123)).toBe('١٢٣');
  });

  it('converts a string input', () => {
    expect(toArabicDigits('456')).toBe('٤٥٦');
  });

  it('handles zero', () => {
    expect(toArabicDigits(0)).toBe('٠');
  });

  it('leaves non-digit characters unchanged', () => {
    expect(toArabicDigits('abc')).toBe('abc');
  });

  it('falls back to original digit when not in ARABIC_DIGITS_MAP (edge case)', () => {
    expect(toArabicDigits('0123456789').length).toBe(10);
  });

  it('converts all ten digits', () => {
    expect(toArabicDigits('0123456789')).toBe('٠١٢٣٤٥٦٧٨٩');
  });
});

describe('formatArabicNumber', () => {
  test.each([
    [0, '٠'],
    [7, '٧'],
    [999, '٩٩٩'],
    [1338, '١٬٣٣٨'],
    [39456, '٣٩٬٤٥٦'],
    [1234567, '١٬٢٣٤٬٥٦٧'],
  ])('renders %d in Arabic digits with the thousands separator', (value, expected) => {
    expect(formatArabicNumber(value)).toBe(expected);
  });
});

describe('sanitizeArabicInput', () => {
  it('strips non-Arabic characters and collapses whitespace', () => {
    expect(sanitizeArabicInput('hello يا صديقي  123')).toBe('يا صديقي ');
  });

  it('leaves nothing behind when the pasted text has no Arabic', () => {
    expect(sanitizeArabicInput('hello world')).toBe('');
  });

  it('strips invisible bidi marks that browsers add when copying Arabic text', () => {
    const pasted = '‏يا صديقي‎ الحبيب‪';
    expect(sanitizeArabicInput(pasted)).toBe('يا صديقي الحبيب');
  });
});

describe('stripInputNoise', () => {
  it('drops invisible bidi marks so they match sanitizeArabicInput output', () => {
    const pasted = '‏يا صديقي‎ الحبيب‪';
    expect(stripInputNoise(pasted)).toBe(sanitizeArabicInput(pasted));
  });

  it('still collapses whitespace runs without treating them as foreign input', () => {
    expect(stripInputNoise('يا   صديقي')).toBe('يا صديقي');
  });

  it('drops leading whitespace the same way sanitizeArabicInput does', () => {
    expect(stripInputNoise('  يا صديقي')).toBe(sanitizeArabicInput('  يا صديقي'));
  });
});

describe('formatArabicCount', () => {
  const carForms: ArabicNounForms = {
    singular: 'سيارة',
    dual: 'سيارتان',
    plural: 'سيارات',
  };

  test.each([
    [0, 'لا سيارة'],
    [1, 'سيارة'],
    [2, 'سيارتان'],
    [5, '٥ سيارات'],
    [10, '١٠ سيارات'],
    [11, '١١ سيارة'],
    [15, '١٥ سيارة'],
    [-3, '٣ سيارات'],
    [100, '١٠٠ سيارة'],
    [102, '١٠٢ سيارة'],
    [103, '١٠٣ سيارات'],
    [110, '١١٠ سيارات'],
    [111, '١١١ سيارة'],
    [1005, '١٬٠٠٥ سيارات'],
  ])('handles count %d', (count, expected) => {
    expect(formatArabicCount({ count, nounForms: carForms })).toBe(expected);
  });

  test.each([
    [3.7, '٣٫٧ سيارة'],
    [-2.5, '٢٫٥ سيارة'],
    [0.1, '٠٫١ سيارة'],
  ])('renders non-integer count %d as singular', (count, expected) => {
    expect(formatArabicCount({ count, nounForms: carForms })).toBe(expected);
  });
});

describe('formatArabicCount with VERSES_NOUN_FORMS', () => {
  test.each([
    [1, 'بيت'],
    [2, 'بيتان'],
    [3, '٣ أبيات'],
    [10, '١٠ أبيات'],
    [11, '١١ بيت'],
    [100, '١٠٠ بيت'],
    [1338, '١٬٣٣٨ بيت'],
  ])('renders %d verses', (count, expected) => {
    expect(formatArabicCount({ count, nounForms: VERSES_NOUN_FORMS })).toBe(expected);
  });
});

describe('stripTashkeel', () => {
  it('removes harakat and tatweel but keeps the letters', () => {
    expect(stripTashkeel('المُتَنَبِّي')).toBe('المتنبي');
  });

  it('removes the tatweel elongation mark', () => {
    expect(stripTashkeel('كتــاب')).toBe('كتاب');
  });

  it('leaves unvocalized text unchanged', () => {
    expect(stripTashkeel('قافية')).toBe('قافية');
  });
});

describe('NON_ARABIC_BASIC_REGEX', () => {
  it('matches only the basic Arabic block plus whitespace', () => {
    expect('حب'.replace(NON_ARABIC_BASIC_REGEX, '')).toBe('حب');
    expect('abc'.replace(NON_ARABIC_BASIC_REGEX, '')).toBe('');
  });
});
