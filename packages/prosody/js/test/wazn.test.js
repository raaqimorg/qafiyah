import assert from 'node:assert/strict';
import { test } from 'node:test';

import { BahrName, ShatrHalf } from '../enums.js';
import { BahrSearch } from '../wazn/bahr-search.js';

test('a wafer bayt is detected as الوافر with its feet named per hemistich', () => {
  const search = new BahrSearch({
    shatrsOfPoem: ['أَلَا لَا يَجْهَلَنْ أَحَدٌ عَلَيْنَا', 'فَنَجْهَلَ فَوْقَ جَهْلِ الْجَاهِلِينَا'],
  });
  search.setResultOfShatr();
  assert.equal(search.result.length, 1);
  const [bahr] = search.result;
  assert.equal(bahr.whichBahr, BahrName.wafer);
  assert.deepEqual(bahr.taffelahLabels(ShatrHalf.first), ['مفاعلْتنْ', 'مفاعلتن', 'فعولن']);
  assert.deepEqual(bahr.taffelahLabels(ShatrHalf.second), ['مفاعلتن', 'مفاعلْتنْ', 'فعولن']);
});

test('a single hemistich is measured against itself', () => {
  const search = new BahrSearch({ shatrsOfPoem: ['أَلَا لَا يَجْهَلَنْ أَحَدٌ عَلَيْنَا'] });
  search.setResultOfShatr();
  assert.equal(search.result[0].whichBahr, BahrName.wafer);
});

test('a bayt that fits no bahr yields one placeholder result named لا نتيجة', () => {
  const search = new BahrSearch({ shatrsOfPoem: ['بَبَبَبَبَبَبَبَبْ', 'بَبَبَبَبَبَبَبَبْ'] });
  search.setResultOfShatr();
  assert.equal(search.result.length, 1);
  assert.equal(search.result[0].whichBahr, BahrName.noResult);
});
