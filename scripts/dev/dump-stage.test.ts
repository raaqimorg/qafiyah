import { describe, expect, test } from 'bun:test';

import { dumpStageLine } from './dump-stage';

describe('dumpStageLine', () => {
  test('names the dump that pull used', () => {
    const stderr =
      '[data-pull] downloading db/0046_15_10_2026...\n[data-pull] using db/0046_15_10_2026\n';
    expect(dumpStageLine(0, stderr)).toEqual({ ok: true, text: 'using 0046_15_10_2026' });
  });

  test('says there is no token', () => {
    const stderr = '[data-pull] no R2 token in .env, nothing to download\n';
    expect(dumpStageLine(0, stderr)).toEqual({
      ok: true,
      text: 'no R2 token, using local dumps or the sample',
    });
  });

  test('warns and goes on when the download fails', () => {
    const stderr = '[data-pull] checksum mismatch in db/0046_15_10_2026\n';
    expect(dumpStageLine(1, stderr)).toEqual({
      ok: false,
      text: 'download failed, using local dumps or the sample',
    });
  });
});
