import { readdirSync } from 'node:fs';

export const SAMPLE_DUMP = '0000_default';

const SNAPSHOT_FOLDER = /^\d{4}_\d{2}_\d{2}_\d{4}$/;

export type DumpChoice =
  | { readonly kind: 'sample' }
  | { readonly kind: 'snapshot'; readonly folder: string }
  | { readonly kind: 'invalid'; readonly reason: string };

export function dumpChoice(argv: readonly string[]): DumpChoice {
  const index = argv.indexOf('--dump');
  if (index === -1) return { kind: 'sample' };
  const folder = argv[index + 1];
  if (folder === undefined || folder.startsWith('--')) {
    return {
      kind: 'invalid',
      reason: 'pass a dump folder after --dump, for example --dump 0046_15_10_2026',
    };
  }
  if (folder === SAMPLE_DUMP) return { kind: 'sample' };
  if (!SNAPSHOT_FOLDER.test(folder)) {
    return { kind: 'invalid', reason: `${folder} is not a dump folder name (NNNN_DD_MM_YYYY)` };
  }
  return { kind: 'snapshot', folder };
}

export function holdsDump(dir: string): boolean {
  try {
    return readdirSync(dir).some((name) => name.endsWith('.dump') || name.includes('.dump.part-'));
  } catch {
    return false;
  }
}
