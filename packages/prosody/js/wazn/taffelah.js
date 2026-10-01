import { slice } from '../strings.js';

import { variantsOfBase } from './taffelah-variants.js';

export class Taffelah {
  start = 0;
  end = undefined;
  matched = null;
  whereError = 0;
  find = false;

  constructor({ baseTaffelah, allowedVariants }) {
    const licensed = variantsOfBase.get(baseTaffelah);
    const unlicensed = allowedVariants.filter((variant) => !licensed.includes(variant));
    if (unlicensed.length > 0) {
      throw new Error(
        `${baseTaffelah.key} does not license ${unlicensed.map((v) => v.key).join(', ')}`
      );
    }
    this.baseTaffelah = baseTaffelah;
    this.allowedVariants = allowedVariants;
  }

  findTaffelah(numKetaba, write) {
    if (!this.find) {
      this.whereError = 0;
    }
    this.find = false;

    for (const variant of this.allowedVariants) {
      const bits = variant.bits;
      if (this.start + bits.length <= numKetaba.length) {
        if (bits === slice(numKetaba, this.start, this.start + bits.length)) {
          this.end = this.start + bits.length;
          this.matched = variant;
          this.find = true;
        } else if (write) {
          this.find = false;
          this.whereError = 1;
        }
      } else {
        const remaining = numKetaba.length - this.start;
        if (slice(bits, 0, remaining) !== slice(numKetaba, this.start, numKetaba.length)) {
          this.whereError = 1;
        } else if (write) {
          if (
            slice(bits, 0, remaining - 1) !== slice(numKetaba, this.start, numKetaba.length - 1)
          ) {
            this.whereError = 1;
          }
        }
      }
    }

    if (!this.find) {
      this.setError(numKetaba);
      this.matched = null;
    }
  }

  setError(numKetaba) {
    for (const variant of this.allowedVariants) {
      if (this.start + variant.bits.length <= numKetaba.length) {
        const remaining = numKetaba.length - this.start;
        try {
          const matchesLabel =
            slice(numKetaba, this.start, numKetaba.length) === slice(variant.label, 0, remaining) &&
            !this.find;
          if (!matchesLabel) {
            this.whereError = numKetaba.length;
          }
        } catch (error) {
          if (!(error instanceof RangeError)) throw error;
        }
      }
    }
  }
}
