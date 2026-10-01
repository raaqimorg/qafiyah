import { ShatrHalf } from '../enums.js';

export class BahrPattern {
  constructor({ name, first, second, maxLength }) {
    this.name = name;
    this.first = first;
    this.second = second;
    this.maxLength = maxLength;
  }

  taffelahs(half) {
    return half === ShatrHalf.first ? this.first : this.second;
  }
}
