const FALLBACK = 'using local dumps or the sample';

export function dumpStageLine(code: number, stderr: string): { ok: boolean; text: string } {
  if (code !== 0) return { ok: false, text: `download failed, ${FALLBACK}` };
  const used = /\[data-pull] using [a-z]+\/(\S+)/.exec(stderr);
  if (used) return { ok: true, text: `using ${used[1]}` };
  if (stderr.includes('[data-pull] no R2 token'))
    return { ok: true, text: `no R2 token, ${FALLBACK}` };
  return { ok: true, text: FALLBACK };
}
