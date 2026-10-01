import { HarakahWeight } from '../enums.js';

import {
  alef,
  alefHamza,
  damma,
  dammatan,
  fatha,
  fathatan,
  kasra,
  kasratan,
  shadda,
  sukun,
  waw,
  wawHamza,
  yeh,
  yehHamza,
} from './arabic-chars.js';

const weights = new Map([
  [fatha, HarakahWeight.moving],
  [kasra, HarakahWeight.moving],
  [damma, HarakahWeight.moving],
  [fathatan, HarakahWeight.movingThenStill],
  [kasratan, HarakahWeight.movingThenStill],
  [dammatan, HarakahWeight.movingThenStill],
  [sukun, HarakahWeight.still],
  [shadda, HarakahWeight.stillThenMoving],
  [alefHamza, HarakahWeight.hamza],
  [wawHamza, HarakahWeight.hamza],
  [yehHamza, HarakahWeight.hamza],
]);

export const typeOf = (c) => weights.get(c) ?? HarakahWeight.none;

export const bitsOf = (list) => list.map((weight) => weight.bits).join('');

const tanweenVowels = new Map([
  [fathatan, fatha],
  [kasratan, kasra],
  [dammatan, damma],
]);

export const baseVowelOfTanween = (c) => tanweenVowels.get(c) ?? '';

const madForVowel = new Map([
  [fatha, `${alef}${sukun}`],
  [kasra, `${yeh}${sukun}`],
  [damma, `${waw}${sukun}`],
]);

export const noMadLetter = '-1';

export const madLetterFor = (vowel) => madForVowel.get(vowel) ?? noMadLetter;
