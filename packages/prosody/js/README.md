# Wazn/Qafiyah detector (JavaScript port)

This is a line-for-line port of the original Dart engine, written for a Flutter app and not included in this repo. It uses plain ES modules with no dependencies, for Node 22 or later. `../AGENTS.md` explains how the engine works. The files here mirror the Dart ones by name (`arowd_converter.dart` becomes `converter/arowd-converter.js`, and so on). `wazn/bahr-patterns.js` was generated mechanically from `bahr_patterns.dart`, so none of its 122 foot slots were typed by hand.

```js
import { BahrSearch, ShatrHalf, QafiyahTerm } from "./index.js";

const search = new BahrSearch({ shatrsOfPoem: ["صدر البيت", "عجز البيت"] });
search.setResultOfShatr();
for (const bahr of search.result) {
  bahr.whichBahr.label;
  bahr.taffelahLabels(ShatrHalf.first);
  bahr.rhymeResult.get(QafiyahTerm.rawi);
}
```

## Differences from the Dart code

- `Bahr.setRhyme` does not render Flutter widgets (`rhymeResultView`), and `findBahr` and `setResultOfShatr` take no `BuildContext`. Everything else keeps the original behaviour, including the quirks listed in `../AGENTS.md`.
- Dart throws a `RangeError` when a string index or substring is out of range, but JavaScript quietly returns `undefined` or clamps. `strings.js` (`charAt` and `slice`) restores the Dart behaviour, so input that crashes the Dart engine also throws in this port.

## Input rules

- Give one hemistich per string, alternating صدر and عجز. In the fixture, a bayt is written as `صدر*عجز` and split on `*`.
- Write full تشكيل. Letters without harakat are guessed by heuristics.
- Only `,`, `،`, tatweel, and dagger alef are removed. Any other punctuation (`؟`, `!`, `.`, quotes) is read as a letter.
- An empty or whitespace-only hemistich throws a `RangeError`.

## Tests and evaluation

```sh
npm test                  # 109 tests: README examples + parity with Dart on 100 bayt
node scripts/evaluate.js  # accuracy against the catalog labels, writes test/report.md
```

- `test/fixtures/sample-100-bayt.json` holds the first bayt of each of the 100 poems in `data/db/0000_default` (the plaintext CC0 sample dump), with each poem's catalog meter, rhyme letter, and majra.
- `test/fixtures/dart-golden-100-bayt.json` is the output of the original Dart engine on that fixture. It was produced by a copy of the Dart engine with the Flutter rendering removed. `dart-parity.test.js` checks that the port reproduces it exactly.
