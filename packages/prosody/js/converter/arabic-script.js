import { alef, alefMaddah, alefMaksura, diacritics, feh, lam, waw, yeh } from './arabic-chars.js';

export const isLetter = (c) => !diacritics.has(c);

export const isAlefMaddah = (c) => c === alefMaddah;

export const isDefiniteArticle = (c, next) => c === alef && next === lam;

export const sunLetters = new Set([
  'ت',
  'ث',
  'د',
  'ذ',
  'ر',
  'ز',
  'س',
  'ش',
  'ص',
  'ض',
  'ط',
  'ظ',
  'ن',
  'ل',
]);

export const moonLetters = new Set([
  'أ',
  'إ',
  'ؤ',
  'ا',
  'ب',
  'ج',
  'ح',
  'خ',
  'ع',
  'غ',
  'ف',
  'ق',
  'ك',
  'م',
  'ه',
  'و',
  'ي',
]);

export const isSunLetter = (c) => sunLetters.has(c);

export const isMoonLetter = (c) => moonLetters.has(c);

export const isElidedAlefContext = (prev, prev2) => {
  if ((prev === feh || prev === waw) && prev2 === ' ') return true;
  return prev === ' ';
};

export const madLetters = new Set([alef, waw, yeh, alefMaksura]);

export const isMadLetter = (c) => madLetters.has(c);
