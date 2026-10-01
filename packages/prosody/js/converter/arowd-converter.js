import { HarakahWeight } from '../enums.js';
import { charAt } from '../strings.js';

import {
  alef,
  alefHamza,
  alefMaksura,
  fatha,
  hamza,
  heh,
  kasra,
  lam,
  noon,
  sukun,
  teh,
  tehMarbuta,
  waw,
  wawHamza,
  yehHamza,
} from './arabic-chars.js';
import { normalize } from './arabic-normalizer.js';
import {
  isAlefMaddah,
  isDefiniteArticle,
  isElidedAlefContext,
  isLetter,
  isMadLetter,
  isMoonLetter,
  isSunLetter,
} from './arabic-script.js';
import { baseVowelOfTanween, bitsOf, madLetterFor, typeOf } from './harakat.js';

const pad = 5;

const convert = (input, arowdi) => {
  const padding = ' '.repeat(pad);
  const shatr = `${padding}${normalize(input)}${padding}`;
  const lastRealIndex = shatr.length - pad - 2;
  let binary = '';
  let arowdiWriting = '';

  for (let i = pad; i < shatr.length; i++) {
    if (charAt(shatr, i) === ' ') {
      if (i > pad && i <= lastRealIndex) {
        arowdiWriting += ' ';
      }
      continue;
    }

    const prev2 = charAt(shatr, i - 2);
    const prev = charAt(shatr, i - 1);
    const cur = charAt(shatr, i);
    const next = charAt(shatr, i + 1);
    const next2 = charAt(shatr, i + 2);
    const next3 = charAt(shatr, i + 3);
    const next4 = charAt(shatr, i + 4);

    if (!isLetter(cur)) {
      continue;
    }

    if (isLetter(next)) {
      if (isAlefMaddah(cur)) {
        binary += HarakahWeight.movingThenStill.bits;
        arowdiWriting += cur;
      } else if (cur === alefHamza || cur === wawHamza || cur === hamza || cur === yehHamza) {
        binary += HarakahWeight.moving.bits;
        arowdiWriting += `${cur}${fatha}`;
      } else if (cur === lam && next === lam && isLetter(next2)) {
        if (isSunLetter(next2)) {
          arowdiWriting += `${cur}${kasra}${next2}${sukun}${next2}`;
          binary += bitsOf([HarakahWeight.moving, HarakahWeight.still, HarakahWeight.moving]);
          if (
            typeOf(next4) === HarakahWeight.moving &&
            typeOf(next3) === HarakahWeight.stillThenMoving
          ) {
            arowdiWriting += next4;
          } else {
            arowdiWriting += fatha;
          }
          i += 2;
        } else if (isMoonLetter(next2)) {
          arowdiWriting += `${cur}${kasra}${next}${sukun}${next2}`;
          binary += bitsOf([HarakahWeight.moving, HarakahWeight.still, HarakahWeight.moving]);
          arowdiWriting += isLetter(next3) ? fatha : next3;
          i += 2;
        }
      } else if (cur === alef) {
        if (isDefiniteArticle(cur, next) && isLetter(next2)) {
          if (isSunLetter(next2)) {
            if (i === pad) {
              arowdiWriting += alefHamza;
              binary += HarakahWeight.moving.bits;
            }
            arowdiWriting += `${next2}${sukun}${next2}`;
            binary += HarakahWeight.stillThenMoving.bits;
            if (
              typeOf(next4) === HarakahWeight.moving &&
              typeOf(next3) === HarakahWeight.stillThenMoving
            ) {
              arowdiWriting += next4;
            } else {
              arowdiWriting += fatha;
            }
            i += 2;
          } else if (isMoonLetter(next2)) {
            if (i === pad) {
              arowdiWriting += alefHamza;
              binary += HarakahWeight.moving.bits;
            }
            arowdiWriting += `${next}${sukun}${next2}`;
            binary += HarakahWeight.stillThenMoving.bits;
            arowdiWriting += isLetter(next3) ? fatha : next3;
            i += 2;
          } else {
            arowdiWriting += `${cur}${fatha}`;
            binary += HarakahWeight.still.bits;
          }
        } else if ((prev === waw && next === ' ') || i === lastRealIndex) {
          i++;
        } else if (!isDefiniteArticle(next2, next3)) {
          if (isElidedAlefContext(prev, prev2)) {
            if (prev === ' ' && prev2 === ' ') {
              arowdiWriting += `${cur}${fatha}`;
              binary += HarakahWeight.moving.bits;
            }
          } else if (!(next2 === alef && next === ' ')) {
            arowdiWriting += `${cur}${sukun}`;
            binary += HarakahWeight.still.bits;
          }
        }
      } else if (isMadLetter(cur) && cur !== alef) {
        if (cur === waw && next === alef && next2 === ' ' && isDefiniteArticle(next3, next4)) {
          i++;
        } else if (!(isDefiniteArticle(next2, next3) && next === ' ')) {
          if (isMadLetter(next) && i !== lastRealIndex) {
            arowdiWriting += `${cur}${fatha}`;
            binary += HarakahWeight.moving.bits;
          } else if (prev === ' ') {
            arowdiWriting += `${cur}${fatha}`;
            binary += HarakahWeight.moving.bits;
          } else if (isLetter(next)) {
            arowdiWriting += `${cur}${sukun}`;
            binary += HarakahWeight.still.bits;
          }
        }
      } else if (
        (next === alef || next === alefMaksura) &&
        typeOf(next2) === HarakahWeight.movingThenStill
      ) {
        arowdiWriting += `${cur}${baseVowelOfTanween(next2)}${noon}${sukun}`;
        binary += HarakahWeight.movingThenStill.bits;
        i++;
      } else if (isMadLetter(next) && typeOf(next2) !== HarakahWeight.moving) {
        arowdiWriting += `${cur}${fatha}`;
        binary += HarakahWeight.moving.bits;
      } else if (prev === ' ') {
        arowdiWriting += `${cur}${fatha}`;
        binary += HarakahWeight.moving.bits;
      } else {
        arowdiWriting += `${cur}${sukun}`;
        binary += HarakahWeight.still.bits;
      }
    } else if (
      typeOf(prev) === HarakahWeight.moving &&
      cur === heh &&
      typeOf(next) === HarakahWeight.moving &&
      next2 === ' ' &&
      !isDefiniteArticle(next3, next4)
    ) {
      arowdiWriting += `${cur}${next}${madLetterFor(next)}`;
      binary += HarakahWeight.movingThenStill.bits;
      i++;
    } else if (typeOf(next) === HarakahWeight.movingThenStill) {
      const base = baseVowelOfTanween(next);
      arowdiWriting +=
        cur !== tehMarbuta ? `${cur}${base}${noon}${sukun}` : `${teh}${base}${noon}${sukun}`;
      binary += typeOf(next).bits;
      if ((next2 === alef || next2 === alefMaksura) && next3 === ' ') {
        i++;
      }
      i++;
    } else if (typeOf(next) === HarakahWeight.stillThenMoving) {
      arowdiWriting += `${cur}${sukun}${cur}`;
      binary += typeOf(next).bits;
      if (typeOf(next2) === HarakahWeight.movingThenStill) {
        binary += HarakahWeight.still.bits;
        arowdiWriting += `${baseVowelOfTanween(next2)}${noon}${sukun}`;
        if (next3 === alef && charAt(shatr, i + 5) === ' ') {
          i++;
        }
        i++;
      } else if (typeOf(next2) === HarakahWeight.moving) {
        arowdiWriting += next2;
        i++;
      } else {
        arowdiWriting += fatha;
      }
      i++;
    } else {
      arowdiWriting += `${cur}${next}`;
      binary += typeOf(next).bits;
      i++;
    }
  }

  if (charAt(binary, binary.length - 1) === HarakahWeight.moving.bits) {
    binary += HarakahWeight.still.bits;
    arowdiWriting += madLetterFor(charAt(arowdiWriting, arowdiWriting.length - 1));
  }

  return arowdi ? arowdiWriting : binary;
};

export const toArowdiWriting = (shatr) => convert(shatr, true);

export const toBinary = (shatr) => convert(shatr, false);
