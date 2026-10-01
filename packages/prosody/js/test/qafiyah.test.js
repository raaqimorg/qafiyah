import assert from 'node:assert/strict';
import { test } from 'node:test';

import { toArowdiWriting, toBinary } from '../converter/arowd-converter.js';
import { QafiyahTerm } from '../enums.js';
import { Qafiyah } from '../qafiyah/qafiyah.js';

const analyse = (shatr) => {
  const qafiyah = new Qafiyah({
    arowdiWriting: toArowdiWriting(shatr),
    binary: toBinary(shatr),
    shatr,
  });
  qafiyah.analyse();
  return qafiyah.result;
};

test('the qafiyah of a hemistich ending in ينا is a mutawatir with a noon rawi and a yaa radf', () => {
  const result = analyse('فَنَجْهَلَ فَوْقَ جَهْلِ الْجَاهِلِينَا');
  assert.equal(result.get(QafiyahTerm.qafiyah), 'لِيْنَاْ');
  assert.equal(result.get(QafiyahTerm.weight), '/0/0');
  assert.equal(result.get(QafiyahTerm.nickname), 'متواتر');
  assert.equal(result.get(QafiyahTerm.rawi), 'نون مفتوحة');
  assert.equal(result.get(QafiyahTerm.radf), 'ياء');
  assert.equal(result.get(QafiyahTerm.rawiHarakah), 'مطلق');
  assert.equal(result.get(QafiyahTerm.majra), 'مفتوح');
});
