import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { DEFAULT_SOUND_CLASSES, lettersOnly, type SoundClassTable } from './arabic-text';
import { anchorRecall, isAnchor, syntheticFalsePositiveRate } from './calibration';
import {
  ajuzHypotheses,
  classifyStructure,
  parseRows,
  provenancedHemistichs,
  type AjuzHypothesisName,
} from './hemistichs';
import { dominantRawi, rhymeScore, sharedLiteralSuffixLength } from './qafiya';
import { coefficientOfVariation, madOutlierShare } from './regularity';
import {
  assess,
  CALIBRATION_BOUNDS,
  type Assessment,
  type PoemEvidence,
  type Verdict,
} from './verdict';

const CLASSICAL_LABEL = 'amudi';
const TWO_HALF_LABELS: ReadonlySet<string> = new Set([CLASSICAL_LABEL, 'nabati']);
const FIELD_COUNT = 6;

export type CorpusRow = {
  readonly id: string;
  readonly slug: string;
  readonly poemType: string;
  readonly meter: string;
  readonly rhyme: string;
  readonly body: string;
};

export type PoemAssessment = {
  readonly evidence: PoemEvidence;
  readonly assessment: Assessment;
  readonly hypothesis: AjuzHypothesisName | null;
  readonly madOutliers: number;
  readonly parityWarning: boolean;
};

export function parseCorpusLine(line: string): CorpusRow | null {
  if (line.trim().length === 0) return null;
  const fields = line.split('\t');
  if (fields.length < FIELD_COUNT) return null;
  const [id, slug, poemType, meter, rhyme, body] = fields as [
    string,
    string,
    string,
    string,
    string,
    string,
  ];
  return { id, slug, poemType, meter, rhyme, body };
}

export function assessPoem(row: CorpusRow, table: SoundClassTable): PoemAssessment {
  const rows = parseRows(row.body);
  const structure = classifyStructure(rows);
  const lengths = provenancedHemistichs(rows, structure)
    .map((hemistich) => lettersOnly(hemistich).length)
    .filter((length) => length > 0);

  const hypotheses = ajuzHypotheses(rows, structure);
  let best: { readonly name: AjuzHypothesisName; readonly ajuz: readonly string[] } | null = null;
  let bestScore = -1;
  for (const hypothesis of hypotheses) {
    const score = rhymeScore(hypothesis.ajuz, table);
    if (score > bestScore) {
      bestScore = score;
      best = hypothesis;
    }
  }

  const evidence: PoemEvidence = {
    structure,
    ajuzCount: best?.ajuz.length ?? 0,
    rhymeScore: best === null ? 0 : bestScore,
    sharedSuffixLength: best === null ? 0 : sharedLiteralSuffixLength(best.ajuz),
    coefficientOfVariation: coefficientOfVariation(lengths),
    usableHemistichs: lengths.length,
  };

  return {
    evidence,
    assessment: assess(evidence),
    hypothesis: best?.name ?? null,
    madOutliers: madOutlierShare(lengths),
    parityWarning: lengths.length % 2 === 1,
  };
}

export function proposedLabel(row: CorpusRow, verdict: Verdict): string | null {
  if (verdict === 'verified' && !TWO_HALF_LABELS.has(row.poemType)) return CLASSICAL_LABEL;
  return null;
}

const DEFAULT_OUT_DIR = 'reports/corpus';
const REVIEW_PER_BUCKET = 20;
const ANCHOR_CAP = 20000;
const POOL_CAP = 200000;
const SYNTHETIC_TRIALS = 10000;
const SYNTHETIC_SEED = 20260920;
const VERSE_FENCE = '~~~';

type Totals = Map<string, Map<Verdict, number>>;

function bump(totals: Totals, label: string, verdict: Verdict): void {
  const counts = totals.get(label) ?? new Map<Verdict, number>();
  counts.set(verdict, (counts.get(verdict) ?? 0) + 1);
  totals.set(label, counts);
}

function csvCell(value: string): string {
  return value.includes(',') || value.includes('"') ? `"${value.replaceAll('"', '""')}"` : value;
}

function verdictTable(totals: Totals): string {
  const lines = [
    '| label | verified | untestable | contradicted | total |',
    '|---|---|---|---|---|',
  ];
  for (const [label, counts] of [...totals].sort(
    (left, right) =>
      [...right[1].values()].reduce((a, b) => a + b, 0) -
      [...left[1].values()].reduce((a, b) => a + b, 0)
  )) {
    const verified = counts.get('verified') ?? 0;
    const untestable = counts.get('untestable') ?? 0;
    const contradicted = counts.get('contradicted') ?? 0;
    const total = verified + untestable + contradicted;
    const share = (value: number): string =>
      total === 0 ? '0' : `${value} (${((100 * value) / total).toFixed(1)}%)`;
    lines.push(
      `| \`${label}\` | ${share(verified)} | ${share(untestable)} | ${share(contradicted)} | ${total} |`
    );
  }
  return lines.join('\n');
}

async function main(): Promise<void> {
  const input = Bun.argv[2];
  if (input === undefined) {
    console.error('usage: verify-classical.ts <corpus.tsv> [out-dir]');
    process.exit(1);
  }
  const outDir = Bun.argv[3] ?? DEFAULT_OUT_DIR;
  const table = DEFAULT_SOUND_CLASSES;

  const totals: Totals = new Map();
  const reasons = new Map<string, number>();
  const transitions: string[] = [];
  const rhymeAudit: string[] = [];
  const review = new Map<string, string[]>();
  const anchors: string[][] = [];
  const pool: string[] = [];
  let poems = 0;
  let skipped = 0;

  const text = await Bun.file(input).text();
  for (const line of text.split('\n')) {
    const row = parseCorpusLine(line);
    if (row === null) {
      if (line.trim().length > 0) skipped += 1;
      continue;
    }
    poems += 1;

    const result = assessPoem(row, table);
    bump(totals, row.poemType, result.assessment.verdict);
    reasons.set(result.assessment.reason, (reasons.get(result.assessment.reason) ?? 0) + 1);

    const rows = parseRows(row.body);
    const structure = classifyStructure(rows);
    const hypotheses = ajuzHypotheses(rows, structure);
    const ajuz = hypotheses.find((candidate) => candidate.name === result.hypothesis)?.ajuz ?? [];

    if (anchors.length < ANCHOR_CAP && isAnchor(ajuz)) anchors.push([...ajuz]);
    if (pool.length < POOL_CAP && ajuz.length > 0) pool.push(ajuz[0] as string);

    if (result.assessment.verdict === 'verified') {
      const rawi = dominantRawi(ajuz, table);
      if (rawi !== null) rhymeAudit.push(`${row.id},${row.slug},${row.rhyme},${rawi}`);
    }

    const proposed = proposedLabel(row, result.assessment.verdict);
    if (proposed === null) continue;

    transitions.push(
      [
        row.id,
        row.slug,
        row.poemType,
        proposed,
        result.evidence.structure,
        result.hypothesis ?? 'none',
        String(result.evidence.ajuzCount),
        result.evidence.rhymeScore.toFixed(3),
        result.evidence.coefficientOfVariation.toFixed(3),
        result.madOutliers.toFixed(3),
        String(result.parityWarning),
        result.assessment.reason,
      ]
        .map((cell) => csvCell(cell))
        .join(',')
    );

    const bucket = `${row.poemType} -> ${proposed}`;
    const samples = review.get(bucket) ?? [];
    if (samples.length < REVIEW_PER_BUCKET) {
      samples.push(
        [
          `### ${row.slug} (id ${row.id}), ${bucket}`,
          '',
          `reason \`${result.assessment.reason}\`, ajuz ${result.evidence.ajuzCount}, ` +
            `rhyme ${result.evidence.rhymeScore.toFixed(3)}, ` +
            `cv ${result.evidence.coefficientOfVariation.toFixed(3)}, ` +
            `meter \`${row.meter}\`, stored rhyme \`${row.rhyme}\``,
          '',
          VERSE_FENCE,
          rows.map((verse) => verse.join('  *  ')).join('\n'),
          VERSE_FENCE,
        ].join('\n')
      );
      review.set(bucket, samples);
    }
  }

  const recall = anchorRecall(anchors, table);
  const falsePositive = syntheticFalsePositiveRate(
    pool,
    CALIBRATION_BOUNDS.syntheticAjuzCount,
    SYNTHETIC_TRIALS,
    table,
    SYNTHETIC_SEED
  );
  const calibrationPassed =
    recall >= CALIBRATION_BOUNDS.minAnchorRecall &&
    falsePositive <= CALIBRATION_BOUNDS.maxSyntheticFalsePositive;

  await mkdir(outDir, { recursive: true });

  await writeFile(
    join(outDir, 'calibration.md'),
    [
      '# Calibration',
      '',
      `Poems read: ${poems}. Unparseable lines skipped: ${skipped}.`,
      '',
      '## Tier A, mechanical anchor',
      '',
      `Anchors found: ${anchors.length} (capped at ${ANCHOR_CAP}).`,
      `Recall: ${recall.toFixed(4)}, bound ${CALIBRATION_BOUNDS.minAnchorRecall}.`,
      '',
      '## Tier B, synthetic negatives',
      '',
      `False positive at ${CALIBRATION_BOUNDS.syntheticAjuzCount} ajuz over ` +
        `${SYNTHETIC_TRIALS} trials: ${falsePositive.toFixed(4)}, ` +
        `bound ${CALIBRATION_BOUNDS.maxSyntheticFalsePositive}.`,
      '',
      `Calibration ${calibrationPassed ? 'PASSED' : 'FAILED'}.`,
      '',
      '## Verdicts by label',
      '',
      verdictTable(totals),
      '',
      '## Reasons',
      '',
      ...[...reasons]
        .sort((left, right) => right[1] - left[1])
        .map(([reason, count]) => `- \`${reason}\`: ${count}`),
      '',
    ].join('\n'),
    'utf8'
  );

  await writeFile(
    join(outDir, 'rhyme-audit.csv'),
    ['id,slug,stored_rhyme,computed_rawi', ...rhymeAudit].join('\n'),
    'utf8'
  );

  await writeFile(
    join(outDir, 'review.md'),
    ['# Review sample', '', ...[...review].flatMap(([, samples]) => samples)].join('\n\n'),
    'utf8'
  );

  if (!calibrationPassed) {
    console.error('calibration outside bounds, refusing to write transitions.csv');
    process.exit(1);
  }

  await writeFile(
    join(outDir, 'transitions.csv'),
    [
      'id,slug,current,proposed,structure,hypothesis,ajuz_count,rhyme_score,cv,mad_outliers,parity_warning,reason',
      ...transitions,
    ].join('\n'),
    'utf8'
  );

  console.log(`wrote ${transitions.length} proposed transitions to ${outDir}`);
}

if (import.meta.main) await main();
