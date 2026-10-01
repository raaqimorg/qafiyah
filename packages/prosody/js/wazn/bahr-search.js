import { toArowdiWriting, toBinary } from '../converter/arowd-converter.js';
import { BahrName, ShatrHalf } from '../enums.js';

import { allBahr } from './bahr-patterns.js';
import { Bahr } from './bahr.js';

export class BahrSearch {
  arowdiKetaba = [];
  numKetaba = [];
  result = [];

  constructor({ shatrsOfPoem }) {
    this.shatrsOfPoem = shatrsOfPoem;
  }

  setLastElemntOfShatr() {
    if (this.shatrsOfPoem.length % 2 !== 0) {
      this.shatrsOfPoem.push(this.shatrsOfPoem[this.shatrsOfPoem.length - 1]);
    }
  }

  setNumKetabaAndArowdiKetaba() {
    this.setLastElemntOfShatr();
    for (const shatr of this.shatrsOfPoem) {
      this.numKetaba.push(toBinary(shatr));
      this.arowdiKetaba.push(toArowdiWriting(shatr));
    }
  }

  setResultOfShatr() {
    if (this.shatrsOfPoem.length === 0) {
      throw new RangeError('shatrsOfPoem is empty');
    }
    if (this.shatrsOfPoem.length <= 1) {
      this.shatrsOfPoem.push(this.shatrsOfPoem[0]);
    } else if (this.shatrsOfPoem[1].length === 0) {
      this.shatrsOfPoem[1] = this.shatrsOfPoem[0];
    }

    this.setNumKetabaAndArowdiKetaba();

    for (let i = 0; i < this.shatrsOfPoem.length; i += 2) {
      this.setBahrResult(
        [this.numKetaba[i], this.numKetaba[i + 1]],
        [this.arowdiKetaba[i], this.arowdiKetaba[i + 1]],
        [this.shatrsOfPoem[i], this.shatrsOfPoem[i + 1]]
      );
    }
  }

  setBahrResult(numKetabaParameter, arwodiKetabaParameter, shatrParameter) {
    let matches = 0;
    let lastCandidate = null;

    for (const [name, pattern] of allBahr) {
      const candidate = new Bahr({
        whichBahr: name,
        firstNumKetaba: numKetabaParameter[0],
        secondNumKetaba: numKetabaParameter[1],
        firstShatr: shatrParameter[0],
        secondShatr: shatrParameter[1],
        firstArowdiShatr: arwodiKetabaParameter[0],
        secondArowdiShatr: arwodiKetabaParameter[1],
      });

      candidate.findBahr();
      candidate.setFind();

      if (numKetabaParameter[0].length > pattern.maxLength) {
        candidate.find = false;
      }

      if (candidate.find) {
        matches++;
        this.result.push(candidate);
      }
      lastCandidate = candidate;
    }

    if (matches === 0 && lastCandidate !== null) {
      lastCandidate.whichBahr = BahrName.noResult;
      lastCandidate.result.set(ShatrHalf.first, []);
      lastCandidate.result.set(ShatrHalf.second, []);
      this.result.push(lastCandidate);
    }
  }
}
