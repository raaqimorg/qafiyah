import { PROD_DOMAIN } from '@qafiyah/config';

type VerseImageOptions = {
  readonly text: string;
  readonly poet: string;
  readonly font: string;
};

const WIDTH = 1080;
const PADDING = 88;
const MAX_FONT = 54;
const MIN_FONT = 28;
const LINE_RATIO = 1.65;
const FOOTER_FONT = 30;

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function fitFontSize(
  ctx: CanvasRenderingContext2D,
  lines: readonly string[],
  font: string
): number {
  const content = WIDTH - PADDING * 2;
  for (let size = MAX_FONT; size > MIN_FONT; size -= 1) {
    ctx.font = `${size}px ${font}`;
    if (lines.every((line) => ctx.measureText(line).width <= content)) return size;
  }
  return MIN_FONT;
}

export async function renderVerseImage({ text, poet, font }: VerseImageOptions): Promise<Blob> {
  await document.fonts.ready;
  const lines = text.split('\n');
  const probe = document.createElement('canvas').getContext('2d');
  if (probe === null) throw new Error('canvas unavailable');
  const fontSize = fitFontSize(probe, lines, font);
  const lineHeight = fontSize * LINE_RATIO;
  const bodyHeight = lines.reduce(
    (sum, line) => sum + (line === '' ? lineHeight / 2 : lineHeight),
    0
  );
  const footerHeight = FOOTER_FONT * 2 * LINE_RATIO;
  const height = Math.round(PADDING * 2 + bodyHeight + footerHeight);

  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const canvas = document.createElement('canvas');
  canvas.width = WIDTH * dpr;
  canvas.height = height * dpr;
  const ctx = canvas.getContext('2d');
  if (ctx === null) throw new Error('canvas unavailable');
  ctx.scale(dpr, dpr);

  ctx.fillStyle = cssVar('--surface') || '#ffffff';
  ctx.fillRect(0, 0, WIDTH, height);

  ctx.direction = 'rtl';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  const center = WIDTH / 2;

  ctx.fillStyle = cssVar('--text') || '#000000';
  ctx.font = `${fontSize}px ${font}`;
  let y = PADDING + fontSize;
  for (const line of lines) {
    if (line === '') {
      y += lineHeight / 2;
      continue;
    }
    ctx.fillText(line, center, y);
    y += lineHeight;
  }

  ctx.fillStyle = cssVar('--text-subtle') || '#676562';
  ctx.font = `${FOOTER_FONT}px ${font}`;
  const footerTop = height - PADDING - FOOTER_FONT;
  ctx.fillText(poet, center, footerTop);
  ctx.fillText(PROD_DOMAIN, center, footerTop + FOOTER_FONT * LINE_RATIO);

  return await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob === null) reject(new Error('image export failed'));
      else resolve(blob);
    }, 'image/png');
  });
}
