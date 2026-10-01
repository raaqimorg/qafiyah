import {
  alef,
  damma,
  fatha,
  heh,
  kasra,
  sukun,
  tehMarbuta,
  waw,
  yeh,
} from '../converter/arabic-chars.js';
import {
  HarakahName,
  QafiyahNickname,
  QafiyahTerm,
  RawiKind,
  qafiyahNicknameForBits,
} from '../enums.js';
import { charAt, slice } from '../strings.js';

import { arabicLetterNames } from './arabic-letter-names.js';

const searchOrder = ['100', '1010', '10110', '101110', '101110'];

export const missing = 'لا يوجد';
export const unknown = 'هناك خطأ';
export const unknownLetter = 'خطأ';
const waslMissing = 'لايوجد';

const isMadd = (char) => [alef, yeh, waw].includes(char);

const isNotRawi = (tail) =>
  ![`${kasra}${yeh}${sukun}`, `${damma}${waw}${sukun}`, `${fatha}${alef}${sukun}`].includes(tail);

const nameOf = (char) => arabicLetterNames.get(char) ?? unknownLetter;

const vowelName = (char) => nameOf(char).replaceAll(tehMarbuta, '');

const rawiKind = (rawi) =>
  rawi.includes(HarakahName.sakinah.label) ? RawiKind.moqayyad : RawiKind.motlaq;

export class Qafiyah {
  constructor({ arowdiWriting, binary, shatr }) {
    this.arowdiWriting = arowdiWriting;
    this.binary = binary;
    this.shatr = shatr;
    this.result = new Map([
      [QafiyahTerm.qafiyah, ''],
      [QafiyahTerm.nickname, ''],
      [QafiyahTerm.weight, ''],
      [QafiyahTerm.rawi, ''],
      [QafiyahTerm.rawiHarakah, ''],
      [QafiyahTerm.wasl, waslMissing],
      [QafiyahTerm.khoroj, ''],
      [QafiyahTerm.taasees, ''],
      [QafiyahTerm.dakheel, ''],
      [QafiyahTerm.radf, ''],
      [QafiyahTerm.majra, missing],
      [QafiyahTerm.tawjeeh, missing],
      [QafiyahTerm.nafath, missing],
      [QafiyahTerm.hathw, missing],
      [QafiyahTerm.eshbaa, missing],
      [QafiyahTerm.rass, missing],
    ]);
  }

  analyse() {
    const weight = this.weightOfQafiyah(this.binary);
    this.result.set(QafiyahTerm.weight, weight);
    this.result.set(QafiyahTerm.nickname, qafiyahNicknameForBits(weight)?.label ?? unknown);
    this.result.set(QafiyahTerm.qafiyah, this.cutQafiyah(this.arowdiWriting, weight.length));
    this.findRawi(this.arowdiWriting, this.shatr);
    this.result.set(QafiyahTerm.weight, weight.replaceAll('1', '/'));
  }

  weightOfQafiyah(binary) {
    const tall = binary.length;
    for (let i = 3; i < 8; i++) {
      if (slice(binary, tall - i, tall) === searchOrder[i - 3]) {
        return slice(binary, tall - i, tall);
      }
    }
    return unknown;
  }

  cutQafiyah(text, symbols) {
    return slice(text, text.length - symbols * 2, text.length);
  }

  findRawi(arowdiWriting, shatr) {
    const ind = arowdiWriting.length;
    const at = (i) => charAt(arowdiWriting, i);
    if (at(ind - 3) !== kasra && at(ind - 2) !== yeh && at(ind - 1) !== sukun) {
      this.fillRawiDetails(arowdiWriting, shatr, ind, 1);
    } else if (at(ind - 3) !== fatha && at(ind - 2) !== waw && at(ind - 1) !== sukun) {
      this.fillRawiDetails(arowdiWriting, shatr, ind, 1);
    } else if (at(ind - 3) !== sukun && at(ind - 2) === heh && at(ind - 1) === sukun) {
      this.fillRawiDetails(arowdiWriting, shatr, ind, 2);
    } else if (at(ind - 2) !== alef && at(ind - 1) !== sukun) {
      this.fillRawiDetails(arowdiWriting, shatr, ind, 2);
    } else {
      this.fillRawiDetails(arowdiWriting, shatr, ind, isMadd(at(ind - 2)) ? 2 : 0);
    }
  }

  fillRawiDetails(arowdiWriting, shatr, ind, minus) {
    const at = (i) => charAt(arowdiWriting, i);
    const result = this.result;

    result.set(
      QafiyahTerm.khoroj,
      at(ind - 2 - minus) === heh
        ? `${nameOf(at(ind - minus))} ${nameOf(at(ind - minus + 1))}`
        : missing
    );

    let rawi;
    if (result.get(QafiyahTerm.nickname) === QafiyahNickname.motaradef.label) {
      rawi = `${nameOf(at(ind - 2))} ${nameOf(at(ind - 1))} `;
    } else if (isNotRawi(slice(arowdiWriting, ind - 3, ind - 1))) {
      rawi = `${nameOf(at(ind - 4))} ${nameOf(at(ind - 3))}`;
    } else {
      rawi = `${nameOf(at(ind - 2 - minus))} ${nameOf(at(ind - 1 - minus))}`;
    }
    result.set(QafiyahTerm.rawi, rawi);

    const last = charAt(shatr, shatr.length - 1);
    if (isMadd(last) || (last === sukun && isMadd(charAt(shatr, shatr.length - 2)))) {
      result.set(
        QafiyahTerm.wasl,
        at(ind - 2 - minus) !== heh
          ? `${nameOf(at(ind - minus))} ${nameOf(at(ind - minus + 1))}`
          : missing
      );
      if (result.get(QafiyahTerm.wasl) === result.get(QafiyahTerm.rawi)) {
        result.set(QafiyahTerm.wasl, waslMissing);
      }
    }

    result.set(
      QafiyahTerm.taasees,
      at(ind - 6 - minus) === alef && at(ind - 5 - minus) === sukun ? alef : missing
    );

    result.set(
      QafiyahTerm.dakheel,
      result.get(QafiyahTerm.taasees) === alef
        ? `${nameOf(at(ind - 4 - minus))} ${nameOf(at(ind - minus - 3))}`
        : missing
    );

    result.set(
      QafiyahTerm.radf,
      at(ind - 3 - minus) === sukun && isMadd(at(ind - 4 - minus))
        ? nameOf(at(ind - 4 - minus))
        : missing
    );

    result.set(QafiyahTerm.rawiHarakah, rawiKind(result.get(QafiyahTerm.rawi)).label);

    if (result.get(QafiyahTerm.rawiHarakah) === RawiKind.motlaq.label) {
      result.set(QafiyahTerm.majra, vowelName(at(ind - minus - 1)));
    } else {
      result.set(QafiyahTerm.tawjeeh, vowelName(at(ind - minus - 1)));
    }

    if (result.get(QafiyahTerm.rawi).includes(arabicLetterNames.get(heh))) {
      result.set(QafiyahTerm.nafath, vowelName(at(ind - minus - 1)));
    }
    if (result.get(QafiyahTerm.radf) !== missing) {
      result.set(QafiyahTerm.hathw, vowelName(at(ind - 5 - minus)));
    }
    if (result.get(QafiyahTerm.dakheel) !== missing) {
      result.set(QafiyahTerm.eshbaa, vowelName(at(ind - minus - 3)));
    }
    if (result.get(QafiyahTerm.taasees) === alef) {
      result.set(QafiyahTerm.rass, vowelName(at(ind - 7 - minus)));
    }
  }
}
