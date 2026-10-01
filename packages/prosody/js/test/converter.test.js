import assert from 'node:assert/strict';
import { test } from 'node:test';

import { toArowdiWriting, toBinary } from '../converter/arowd-converter.js';

test('a fully voweled hemistich is rewritten into arowdi spelling with the final vowel stretched', () => {
  assert.equal(toArowdiWriting('قِفَا نَبْكِ مِنْ ذِكْرَى حَبِيبٍ وَمَنْزِلِ'), 'قِفَاْ نَبْكِ مِنْ ذِكْرَىْ حَبِيْبِنْ وَمَنْزِلِيْ');
});

test('a fully voweled hemistich is encoded as one bit per letter, 1 for moving and 0 for still', () => {
  assert.equal(toBinary('قِفَا نَبْكِ مِنْ ذِكْرَى حَبِيبٍ وَمَنْزِلِ'), '11010110101011010110110');
});

test('tanween is encoded as a moving letter followed by a still noon', () => {
  assert.equal(toArowdiWriting('أَلَا لَا يَجْهَلَنْ أَحَدٌ عَلَيْنَا'), 'أَلَاْ لَاْ يَجْهَلَنْ أَحَدُنْ عَلَيْنَاْ');
  assert.equal(toBinary('أَلَا لَا يَجْهَلَنْ أَحَدٌ عَلَيْنَا'), '1101010110111011010');
});

test('an empty hemistich throws a RangeError, as the Dart original does', () => {
  assert.throws(() => toBinary(''), RangeError);
  assert.throws(() => toBinary('   '), RangeError);
});
