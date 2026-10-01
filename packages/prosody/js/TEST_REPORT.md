# Test Report: JavaScript port of the wazn/qafiyah detector

Date: 2026-09-26. Node v22.12.0.

## 1. Unit tests (`npm test`)

| Result | Count |
| ------ | ----- |
| Tests  | 109   |
| Passed | 109   |
| Failed | 0     |

| File                       | Tests | What it checks                                                                                                                                    |
| -------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test/converter.test.js`   | 4     | The arowdi writing and binary encoding from the Dart README examples, tanween handling, and empty input throwing `RangeError`                     |
| `test/wazn.test.js`        | 3     | الوافر detection with its feet, a single hemistich measured against itself, and the لا نتيجة placeholder                                          |
| `test/qafiyah.test.js`     | 1     | Qafiyah, weight, nickname, rawi, radf, and majra from the Dart README example                                                                     |
| `test/dart-parity.test.js` | 101   | The JS output equals the original Dart output on each of the 100 bayt, plus one test that the Dart output file and the fixture list the same bayt |

## 2. Parity with the original Dart engine

I ran the original Dart code (with its Flutter display code removed) and this JS port on the same inputs, then compared everything: the binary encoding, the arowdi writing, every detected bahr with its feet, and the full qafiyah result.

| Input set                                                                                          | Identical |
| -------------------------------------------------------------------------------------------------- | --------- |
| 100 test bayt (`test/fixtures/sample-100-bayt.json`)                                               | 100 / 100 |
| Every verse in the sample dump plus 14 edge cases (empty, only `*`, Latin text, digits, and so on) | 625 / 625 |

In the 10 cases where Dart throws an exception, JS throws the same kind of error. The port gives the same results as the Dart engine; it does not change how the engine behaves.

## 3. Accuracy against the catalog labels

**Test data:** the first bayt of each of the 100 poems in `data/db/0000_default`. Each bayt keeps its poem's catalog meter, rhyme letter, and majra. Run `node scripts/evaluate.js` to regenerate the full per-bayt tables in `test/report.md`.

| Metric                                                                                  | Result           |
| --------------------------------------------------------------------------------------- | ---------------- |
| Bayt evaluated                                                                          | 100              |
| Engine crashed                                                                          | 0                |
| Catalog meter not one of the engine's 22 bahrs (أحذ الكامل, مخلع البسيط, 3 × غير معروف) | 5                |
| Meter: first detected bahr is correct                                                   | 42 / 95 (44.2%)  |
| Meter: correct bahr anywhere in the results                                             | 42 / 95 (44.2%)  |
| Meter: no bahr detected                                                                 | 52 / 95          |
| Meter: only a wrong bahr detected                                                       | 1 / 95           |
| Meter: more than one bahr detected                                                      | 0 / 95           |
| Rawi matches the catalog rhyme letter                                                   | 94 / 100 (94.0%) |
| Majra matches the catalog (only 14 bayt are labelled)                                   | 13 / 14 (92.9%)  |

### Meter accuracy per catalog meter

| Catalog meter | Bayt | Correct | %      |
| ------------- | ---- | ------- | ------ |
| الطويل        | 32   | 14      | 43.8%  |
| الوافر        | 23   | 10      | 43.5%  |
| الكامل        | 14   | 8       | 57.1%  |
| البسيط        | 10   | 6       | 60.0%  |
| المتقارب      | 6    | 0       | 0.0%   |
| المنسرح       | 3    | 0       | 0.0%   |
| الرجز         | 2    | 1       | 50.0%  |
| الخفيف        | 2    | 1       | 50.0%  |
| الرمل         | 1    | 0       | 0.0%   |
| مجزوء الرمل   | 1    | 1       | 100.0% |
| السريع        | 1    | 1       | 100.0% |

### Qafiyah shapes detected

| Shape                   | Bayt |
| ----------------------- | ---- |
| متواتر                  | 49   |
| متدارك                  | 34   |
| متراكب                  | 11   |
| مترادف                  | 3    |
| هناك خطأ (unrecognised) | 3    |

## 4. Why the meter misses happen

The engine almost never picks a wrong bahr (1 case out of 95). When it fails, it returns لا نتيجة. I checked the misses by encoding the words below directly:

1. **The converter misreads some words.** This is the largest cause.
   - همزة الوصل after a voweled فَ or وَ is read as a long vowel: `فَاعْجِلْ` becomes `10010`, which has two sakin letters in a row and cannot occur. The correct encoding is `1010`.
   - A هاء at the end of a word, between two vowels, is always lengthened as if it were a pronoun: `وَأَكْرَهُ أَنْ` becomes `وَأَكْرَهُوْ`, encoded `11011010` instead of `1101110`.
   - The silent alef after واو الجماعة is read as a voweled و: `تَأْخُذوا` becomes `تَأْخُذَوَ`.
2. **Some زحافات are missing from the bahr table.** In these cases the encoding is correct but no foot pattern fits.
   - الطويل does not allow مفاعيلُ in foot 2. For example, `مَطَالِبُ دُنْيَاهُ بِإِتْعَابِ نَفْسِهِ` scans as فعولُ مفاعيلُ فعولن مفاعلن and is rejected.
   - خرم (a dropped first syllable) is not supported: `إِنْ تَأْخُذوا`, `لَمْ أَرَ`, `أَطْلَقْتُ`.
3. **Typos in the catalog data:** `أَََمِنْ`, `لمَ ْتَكَلَّمِ`, `وّذي`, `لَكِنَّهَأ`. Some bayt are barely voweled (PiOk 22%, UFjX 50%). See section 6.
4. **A word split across the two hemistichs** (تدوير): `وَالْ*أَنْصَابِ`.

## 5. Rawi misses (6)

| Poem | End of عجز  | Catalog | Engine |
| ---- | ----------- | ------- | ------ |
| EFoV | يَسودُها    | د       | هاء    |
| hGYJ | جُودُها     | د       | هاء    |
| jMVu | يَمِينَهَا  | ن       | هاء    |
| qHpH | إِغْفائِها  | ء       | هاء    |
| APjg | الْمَساعِيا | ي       | ألف    |
| Dqvq | النَّوَى    | ا       | واو    |

Five of the six are one rule. When a line ends in `ـها` or `ـيا`, the engine takes the final letter as the rawi, but the catalog uses the letter before it (the هاء or ألف there is وصل). In the sixth (Dqvq), the line ends in an alef maqsura: the engine takes the و before it as the rawi, while the catalog files the poem under ألف. Its detected qafiyah (`َ نْنَوَىْ`) also starts with a stray fatha, which suggests the arowdi writing is misaligned. I have not traced this one further.

## 6. Data quality: is the تشكيل in the input correct?

Mostly yes, but not all of it. I checked all 100 bayt automatically for mark coverage and for patterns that are always wrong (stacked vowels, a mark after a space, شدة with سكون, and so on). I then read the flagged verses by eye. Individual vowels were not proofread across all 100.

### 6.1 Real errors in the data (7 bayt)

| Poem | As written                                                                                   | Should be                                   |
| ---- | -------------------------------------------------------------------------------------------- | ------------------------------------------- |
| gnNg | `أَََمِنْ` (three fathas), `لمَ ْتَكَلَّمِ`, `بحُِوْمَانَةِ` (damma and kasra on one letter) | `أَمِنْ`, `لَمْ تَكَلَّمِ`, `بِحَوْمَانَةِ` |
| cSbN | `وّذي` (shadda on the first letter)                                                          | `وَذِي`                                     |
| aDWM | `حَوْلَنًا`, `يَعْذِفَنَ`                                                                    | `حَوْلَنا`, `يَعْذِفْنَ`                    |
| UFjX | `الُّبابُّ`, `أسِرِتَّه`                                                                     | `اللُّبابُ`, `أَسِرَّتِهِ`                  |
| VaWq | `لَكِنَّهَأ`                                                                                 | `لَكِنَّها`                                 |
| pKFL | `جَديدراً` (an extra letter, not a تشكيل error)                                              | `جَديراً`                                   |

These need to be fixed in the catalog data, not in the engine.

### 6.2 Partly voweled (valid Arabic, but the engine has to guess)

- Across all 100 bayt, 83% of letters carry a mark (bare long-vowel letters are not counted).
- By bayt: 10 have at least 95% of letters marked, 63 have 80 to 95%, 23 have 50 to 80%, and 4 have less than 50%. The worst is PiOk at 22%: `أرى عبدَ عمروٍ قد أساطَ ابنَ عمهِ`.
- 10 bayt contain no sukun at all, written like `مِن`, `كَفَفتُ`, `أَهلِهِ`: cSbN, fYGM, LVLi, oMec, PiOk, pKFL, sHdR, sydF, TbpF, UFjX. This is a common style (the sukun is implied), but the engine has to infer every sakin letter.

### 6.3 A non-standard placement the engine gets wrong (1 bayt)

OIJn writes `الخِلاَجِ`, with the fatha on the alef instead of on the lam (`لَا`). The normalizer deletes `اَ` entirely, so the long vowel disappears (`الْخِلْجِ`).

### 6.4 Correct تشكيل that the engine misreads (engine bugs, not data)

- **Tanween before shadda, as in `عَدِيٍّ` (4 bayt: amUn, krdS, RkOu, yEON).** Tanween before shadda is the standard Unicode (NFC) order, but the normalizer only reorders a plain vowel with shadda, not tanween. So the shadda is lost: `مِنْ عَدِيٍّ` becomes `عَدِيِنْ`, encoded `101110`. With shadda first, the same word encodes correctly as `عَدِيْيِنْ`, `1011010`.
- **The silent و of `عَمْرٍو` (hqxQ)** is read as a sakin letter (`عَمْرِنْوْ`), which adds an extra `0`.

Two things that look odd are actually fine:

- `فَاِمشوا` (a kasra on همزة الوصل, in LVLi and TbpF): the engine drops it, which is how it's pronounced.
- `هَلَ اتَى` (rtNy): moving the hamza's vowel onto the lam is a valid classical form.

### 6.5 What this means for the meter score

Only about 7 to 12 of the 53 meter misses can be blamed on the data (sections 6.1 to 6.3). Most misses come from the engine: the converter misreads in section 4.1, plus the bugs in section 6.4. Because 27 bayt are less than 80% voweled, input should be required to have full تشكيل, or the engine will keep guessing.

## 7. Recommendations

1. Fix the three converter rules in section 4.1. They are the largest source of misses.
2. Make the normalizer reorder tanween + shadda the same way it already reorders a plain vowel + shadda (tanween U+064B to U+064D followed by shadda U+0651 becomes shadda followed by tanween). Also treat the و of `عمرو` as silent, and handle a fatha written on the alef (`لاَ`) instead of deleting the alef.
3. Add the missing زحافات to `bahr-patterns.js` (كف in الطويل, خرم at the start of a line). Also fix the known متكاوس bug (`searchOrder` lists `101110` twice).
4. Treat a final هاء or مد after a moving letter as وصل when choosing the rawi.
5. Fix the 7 bayt in section 6.1 in the catalog data, and state full تشكيل as a required input rule.
6. Once the rules change on purpose, the Dart parity tests will fail by design. Update `test/fixtures/dart-golden-100-bayt.json` and track progress with the accuracy numbers above.
