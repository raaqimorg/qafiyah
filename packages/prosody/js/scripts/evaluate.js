import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { toBinary } from '../converter/arowd-converter.js';
import { BahrName, HarakahName, QafiyahTerm, ShatrHalf } from '../enums.js';
import { arabicLetterNames } from '../qafiyah/arabic-letter-names.js';
import { Bahr } from '../wazn/bahr.js';

import { analyzeBayt } from './analyze-bayt.js';

const fixturePath = fileURLToPath(
  new URL('../test/fixtures/sample-100-bayt.json', import.meta.url)
);
const reportPath = fileURLToPath(new URL('../test/report.md', import.meta.url));

const engineToCatalogMeter = new Map(
  Object.values(BahrName)
    .filter((name) => name !== BahrName.noResult)
    .map((name) => [name.label.replaceAll(' ', ''), name])
);
engineToCatalogMeter.set('المتدارك', BahrName.mohdath);

const bahrForCatalogMeter = (meter) => engineToCatalogMeter.get(meter.replaceAll(' ', '')) ?? null;

const harakahLabels = Object.values(HarakahName).map((name) => name.label);

const rawiLetterName = (rawi) => {
  const words = rawi.trim().split(' ');
  return harakahLabels.includes(words.at(-1)) ? words.slice(0, -1).join(' ') : words.join(' ');
};

const expectedRawiName = (item) => arabicLetterNames.get(item.rhymeLetter) ?? item.rhymeName;

const majraOfCatalog = new Map([
  ['فتحة', 'مفتوح'],
  ['فتحتان', 'مفتوح'],
  ['كسرة', 'مكسور'],
  ['كسرتان', 'مكسور'],
  ['ضمة', 'مضموم'],
  ['ضمتان', 'مضموم'],
  ['سكون', 'مقيد'],
]);

const engineMajra = (rhyme) =>
  rhyme[QafiyahTerm.rawiHarakah.label] === 'مقيد' ? 'مقيد' : rhyme[QafiyahTerm.majra.label];

const failurePoint = (item, bahrName) => {
  const [first, second = first] = item.content.split('*');
  const bahr = new Bahr({
    whichBahr: bahrName,
    firstNumKetaba: toBinary(first),
    secondNumKetaba: toBinary(second),
    firstShatr: first,
    secondShatr: second,
    firstArowdiShatr: '',
    secondArowdiShatr: '',
  });
  bahr.setTaffelah();
  const points = [];
  for (const half of Object.values(ShatrHalf)) {
    const taffelahs = bahr.taffelahOfBahr.get(half);
    const bits = bahr.numKetabaOf(half);
    for (let j = 0; j < taffelahs.length; j++) {
      const taffelah = taffelahs[j];
      taffelah.start = j === 0 ? 0 : taffelahs[j - 1].end;
      taffelah.findTaffelah(bits, false);
      const found = taffelah.matched !== null;
      taffelah.matched = null;
      taffelah.find = false;
      if (!found) {
        points.push(
          `${half.label}: التفعيلة ${j + 1} (${taffelah.baseTaffelah.label}) عند "${bits.slice(taffelah.start)}"`
        );
        break;
      }
    }
    if (points.length === 0 && half === ShatrHalf.first && bits.length > bahr.pattern.maxLength) {
      points.push(`${half.label}: أطول من ${bahr.pattern.maxLength}`);
    }
  }
  return points.length > 0 ? points.join('؛ ') : 'كل التفعيلات طابقت';
};

const evaluate = (items) =>
  items.map((item) => {
    const expectedBahr = bahrForCatalogMeter(item.meter);
    let analysis;
    try {
      analysis = analyzeBayt(item.content);
    } catch (error) {
      return { item, expectedBahr, error: `${error.name}: ${error.message}` };
    }
    const detected = analysis.bahrs
      .map((bahr) => bahr.name)
      .filter((name) => name !== BahrName.noResult.label);
    const rawi = analysis.rhyme[QafiyahTerm.rawi.label];
    const expectedMajra = item.majra ? majraOfCatalog.get(item.majra) : null;
    return {
      item,
      expectedBahr,
      analysis,
      detected,
      top1: expectedBahr !== null && detected[0] === expectedBahr.label,
      anyMatch: expectedBahr !== null && detected.includes(expectedBahr.label),
      rawiOk: rawiLetterName(rawi) === expectedRawiName(item),
      expectedMajra,
      majraOk: expectedMajra ? engineMajra(analysis.rhyme) === expectedMajra : null,
      failure:
        expectedBahr !== null && !detected.includes(expectedBahr.label)
          ? failurePoint(item, expectedBahr)
          : null,
    };
  });

const pct = (count, total) => (total === 0 ? 'n/a' : `${((100 * count) / total).toFixed(1)}%`);

const countBy = (rows, key) =>
  [...rows.reduce((acc, row) => acc.set(key(row), (acc.get(key(row)) ?? 0) + 1), new Map())].sort(
    (a, b) => b[1] - a[1]
  );

const escapeCell = (text) => String(text).replaceAll('|', '\\|').replaceAll('\n', ' ');

const buildReport = (rows) => {
  const covered = rows.filter((row) => row.expectedBahr !== null && !row.error);
  const uncovered = rows.filter((row) => row.expectedBahr === null);
  const errored = rows.filter((row) => row.error);
  const scored = rows.filter((row) => !row.error);
  const top1 = covered.filter((row) => row.top1).length;
  const any = covered.filter((row) => row.anyMatch).length;
  const noResult = covered.filter((row) => row.detected.length === 0).length;
  const wrongOnly = covered.filter((row) => row.detected.length > 0 && !row.anyMatch).length;
  const ambiguous = covered.filter((row) => row.detected.length > 1).length;
  const rawiOk = scored.filter((row) => row.rawiOk).length;
  const majraRows = scored.filter((row) => row.majraOk !== null);
  const majraOk = majraRows.filter((row) => row.majraOk).length;

  const perMeter = countBy(covered, (row) => row.item.meter).map(([meter, total]) => {
    const group = covered.filter((row) => row.item.meter === meter);
    return `| ${meter} | ${total} | ${group.filter((row) => row.top1).length} | ${group.filter((row) => row.anyMatch).length} | ${pct(group.filter((row) => row.anyMatch).length, total)} |`;
  });

  const shapes = countBy(scored, (row) => row.analysis.rhyme[QafiyahTerm.nickname.label]).map(
    ([shape, count]) => `| ${shape} | ${count} |`
  );

  const meterMisses = covered
    .filter((row) => !row.anyMatch)
    .map(
      (row) =>
        `| ${row.item.poemSlug} | ${escapeCell(row.item.content)} | ${row.item.meter} | ${row.detected.join('، ') || 'لا نتيجة'} | ${escapeCell(row.failure)} |`
    );

  const rawiMisses = scored
    .filter((row) => !row.rawiOk)
    .map(
      (row) =>
        `| ${row.item.poemSlug} | ${escapeCell(row.item.content.split('*').at(-1))} | ${row.item.rhymeLetter} (${expectedRawiName(row.item)}) | ${row.analysis.rhyme[QafiyahTerm.rawi.label].trim()} | ${row.analysis.rhyme[QafiyahTerm.qafiyah.label]} |`
    );

  const uncoveredRows = uncovered.map(
    (row) =>
      `| ${row.item.poemSlug} | ${row.item.meter} | ${row.detected?.join('، ') || 'لا نتيجة'} |`
  );

  return `# Wazn/Qafiyah detector: evaluation on 100 bayt

Generated by \`node scripts/evaluate.js\` from \`test/fixtures/sample-100-bayt.json\` (the first bayt of each of the 100 poems in \`data/db/0000_default\`), comparing the detector's output with the catalog's own \`meter\`, \`rhyme\` and \`rhyme_majra\` labels.

## Summary

| Metric | Result |
|---|---|
| Bayt evaluated | ${rows.length} |
| Engine threw an exception | ${errored.length} |
| Catalog meter outside the engine's 22 bahrs | ${uncovered.length} |
| Meter: top-1 correct (first detected bahr) | ${top1} / ${covered.length} (${pct(top1, covered.length)}) |
| Meter: correct bahr anywhere in the results | ${any} / ${covered.length} (${pct(any, covered.length)}) |
| Meter: no bahr detected | ${noResult} / ${covered.length} |
| Meter: only wrong bahrs detected | ${wrongOnly} / ${covered.length} |
| Meter: more than one bahr detected | ${ambiguous} / ${covered.length} |
| Rawi letter matches catalog rhyme letter | ${rawiOk} / ${scored.length} (${pct(rawiOk, scored.length)}) |
| Majra matches catalog (only ${majraRows.length} bayt are labelled) | ${majraOk} / ${majraRows.length} (${pct(majraOk, majraRows.length)}) |

## Meter accuracy per catalog meter

| Catalog meter | Bayt | Top-1 | Any | Any % |
|---|---|---|---|---|
${perMeter.join('\n')}

## Qafiyah shapes detected

| Shape | Bayt |
|---|---|
${shapes.join('\n')}

## Meter misses (${meterMisses.length})

Last column: where the catalog's bahr breaks when this bayt is forced through it.

| Poem | Bayt | Catalog | Detected | First failing foot |
|---|---|---|---|---|
${meterMisses.join('\n')}

## Rawi misses (${rawiMisses.length})

| Poem | Ajuz | Catalog rhyme | Detected rawi | Detected qafiyah |
|---|---|---|---|---|
${rawiMisses.join('\n')}

## Meters the engine does not model (${uncovered.length})

| Poem | Catalog meter | Detected |
|---|---|---|
${uncoveredRows.join('\n')}
`;
};

const items = JSON.parse(readFileSync(fixturePath, 'utf8'));
const report = buildReport(evaluate(items));
writeFileSync(reportPath, report);
process.stdout.write(report.split('## Meter accuracy')[0]);
