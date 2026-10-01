import { ShatrHalf } from '../enums.js';
import { Qafiyah } from '../qafiyah/qafiyah.js';

import { allBahr } from './bahr-patterns.js';

export const noTaffelahLabel = 'error';

export class Bahr {
  rhymeResult = new Map();
  taffelahOfBahr = undefined;
  indexError = 0;
  findForWrite = [false, false];
  error = 0;
  result = new Map([
    [ShatrHalf.first, []],
    [ShatrHalf.second, []],
  ]);
  find = false;

  constructor({
    whichBahr,
    firstNumKetaba,
    secondNumKetaba,
    firstShatr,
    secondShatr,
    firstArowdiShatr,
    secondArowdiShatr,
  }) {
    this.whichBahr = whichBahr;
    this.firstNumKetaba = firstNumKetaba;
    this.secondNumKetaba = secondNumKetaba;
    this.firstShatr = firstShatr;
    this.secondShatr = secondShatr;
    this.firstArowdiShatr = firstArowdiShatr;
    this.secondArowdiShatr = secondArowdiShatr;
  }

  get pattern() {
    return allBahr.get(this.whichBahr);
  }

  numKetabaOf(half) {
    return half === ShatrHalf.first ? this.firstNumKetaba : this.secondNumKetaba;
  }

  taffelahLabels(half) {
    return this.result.get(half).map((variant) => variant?.label ?? noTaffelahLabel);
  }

  setTaffelah() {
    this.taffelahOfBahr = new Map(
      Object.values(ShatrHalf).map((half) => [half, this.pattern.taffelahs(half)])
    );
  }

  findBahr() {
    this.setTaffelah();
    this.find = false;

    for (const half of Object.values(ShatrHalf)) {
      const taffelahs = this.taffelahOfBahr.get(half);
      for (let j = 0; j < taffelahs.length; j++) {
        const taffelah = taffelahs[j];
        taffelah.start = j === 0 ? 0 : taffelahs[j - 1].end;
        taffelah.findTaffelah(this.numKetabaOf(half), false);

        this.result.get(half).push(taffelah.matched);
        if (taffelah.matched === null) {
          this.error++;
          break;
        }

        taffelah.matched = null;
        taffelah.find = false;
      }
    }

    this.setRhyme();
  }

  setRhyme() {
    const qafiyah = new Qafiyah({
      arowdiWriting: this.secondArowdiShatr,
      binary: this.secondNumKetaba,
      shatr: this.secondShatr,
    });
    qafiyah.analyse();
    this.rhymeResult = qafiyah.result;
  }

  setFind() {
    const matchedCount =
      this.result.get(ShatrHalf.first).length + this.result.get(ShatrHalf.second).length;
    if (matchedCount === this.taffelahOfBahr.get(ShatrHalf.first).length * 2 && this.error === 0) {
      this.find = true;
      return true;
    }
    return false;
  }

  setFindShatr() {
    let halfIndex = 0;
    for (const half of Object.values(ShatrHalf)) {
      const taffelahs = this.taffelahOfBahr.get(half);
      let matched = 0;
      for (const taffelah of taffelahs) {
        if (taffelah.find) {
          matched++;
        } else {
          taffelah.setError(this.numKetabaOf(half));
          this.indexError = taffelah.whereError;
        }
      }
      if (matched === taffelahs.length && halfIndex === 0) {
        this.findForWrite[0] = true;
      } else if (matched === taffelahs.length && halfIndex === 1) {
        this.findForWrite[0] = false;
      }
      halfIndex++;
    }
  }
}
