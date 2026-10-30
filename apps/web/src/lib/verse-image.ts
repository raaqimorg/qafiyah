import { err, errAsync, ok, okAsync, type Result, ResultAsync } from 'neverthrow';

import { SHARE_TEXTS } from '@/lib/constants/copy';
import { SITE_WORDMARK_PATH } from '@/lib/constants/site-meta';
import { PROD_DOMAIN } from '@qafiyah/config';

export type VerseCardText = {
  readonly text: string;
  readonly poet: string;
  readonly font: string;
};

type MeasureLine = (line: string, fontSize: number) => number;

type VerseCardLayout = {
  readonly fontSize: number;
  readonly lines: readonly string[];
  readonly height: number;
  readonly pixelRatio: number;
};

const WIDTH = 1080;
const PADDING = 88;
const CONTENT_WIDTH = WIDTH - PADDING * 2;
const MAX_FONT = 54;
const MIN_FONT = 28;
const LINE_RATIO = 1.65;
const FOOTER_FONT = 30;
const MAX_PIXEL_RATIO = 2;
const MAX_HEIGHT = 1920;
const LOGO_HEIGHT = 72;
const LOGO_GAP = 70;

function wrapLine(line: string, fontSize: number, measure: MeasureLine): readonly string[] {
  if (measure(line, fontSize) <= CONTENT_WIDTH) return [line];
  const wrapped: string[] = [];
  let current = '';
  for (const word of line.split(' ')) {
    const candidate = current === '' ? word : `${current} ${word}`;
    if (current !== '' && measure(candidate, fontSize) > CONTENT_WIDTH) {
      wrapped.push(current);
      current = word;
    } else {
      current = candidate;
    }
  }
  wrapped.push(current);
  return wrapped;
}

function cardHeight(lines: readonly string[], fontSize: number): number {
  const lineHeight = fontSize * LINE_RATIO;
  const bodyHeight = lines.reduce(
    (sum, line) => sum + (line === '' ? lineHeight / 2 : lineHeight),
    0
  );
  return Math.round(
    PADDING * 2 + LOGO_HEIGHT + LOGO_GAP + bodyHeight + FOOTER_FONT * 2 * LINE_RATIO
  );
}

export function layoutVerseCard(
  text: string,
  measure: MeasureLine,
  devicePixelRatio: number
): VerseCardLayout | null {
  const source = text.split('\n');
  const pixelRatio = Math.min(devicePixelRatio, MAX_PIXEL_RATIO);
  for (let fontSize = MAX_FONT; fontSize >= MIN_FONT; fontSize -= 1) {
    const fitsWidth =
      fontSize === MIN_FONT || source.every((line) => measure(line, fontSize) <= CONTENT_WIDTH);
    if (!fitsWidth) continue;
    const lines = source.flatMap((line) =>
      line === '' ? [''] : wrapLine(line, fontSize, measure)
    );
    const height = cardHeight(lines, fontSize);
    if (height <= MAX_HEIGHT) return { fontSize, lines, height, pixelRatio };
  }
  return null;
}

function themeColor(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function toError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

function canvasBlob(canvas: HTMLCanvasElement): ResultAsync<Blob, Error> {
  return ResultAsync.fromPromise(
    new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((blob) => {
        if (blob === null) reject(new Error('the canvas could not be exported'));
        else resolve(blob);
      }, 'image/png');
    }),
    toError
  );
}

function loadLogo(): ResultAsync<HTMLImageElement, Error> {
  const logo = new Image();
  logo.src = SITE_WORDMARK_PATH;
  return ResultAsync.fromPromise(logo.decode(), toError).map(() => logo);
}

function drawLogo(
  ctx: CanvasRenderingContext2D,
  logo: HTMLImageElement,
  color: string,
  pixelRatio: number
): Result<void, Error> {
  const width = (LOGO_HEIGHT * logo.naturalWidth) / logo.naturalHeight;
  const tint = document.createElement('canvas');
  tint.width = Math.ceil(width * pixelRatio);
  tint.height = Math.ceil(LOGO_HEIGHT * pixelRatio);
  const tintCtx = tint.getContext('2d');
  if (tintCtx === null) return err(new Error('the logo canvas has no 2d context'));
  tintCtx.drawImage(logo, 0, 0, tint.width, tint.height);
  tintCtx.globalCompositeOperation = 'source-in';
  tintCtx.fillStyle = color;
  tintCtx.fillRect(0, 0, tint.width, tint.height);
  ctx.drawImage(tint, (WIDTH - width) / 2, PADDING, width, LOGO_HEIGHT);
  return ok(undefined);
}

function drawCard(
  canvas: HTMLCanvasElement,
  logo: HTMLImageElement,
  { text, poet, font }: VerseCardText
): ResultAsync<Blob | null, Error> {
  const ctx = canvas.getContext('2d');
  if (ctx === null) return errAsync(new Error('the canvas has no 2d context'));
  const measure: MeasureLine = (line, fontSize) => {
    ctx.font = `${fontSize}px ${font}`;
    return ctx.measureText(line).width;
  };
  const layout = layoutVerseCard(text, measure, window.devicePixelRatio);
  if (layout === null) return okAsync(null);
  const { fontSize, lines, height, pixelRatio } = layout;
  canvas.width = Math.floor(WIDTH * pixelRatio);
  canvas.height = Math.floor(height * pixelRatio);
  ctx.scale(pixelRatio, pixelRatio);
  ctx.fillStyle = themeColor('--surface');
  ctx.fillRect(0, 0, WIDTH, height);
  ctx.direction = 'rtl';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  const center = WIDTH / 2;
  const lineHeight = fontSize * LINE_RATIO;
  const logoDrawn = drawLogo(ctx, logo, themeColor('--text'), pixelRatio);
  if (logoDrawn.isErr()) return errAsync(logoDrawn.error);
  ctx.fillStyle = themeColor('--text');
  ctx.font = `${fontSize}px ${font}`;
  let y = PADDING + LOGO_HEIGHT + LOGO_GAP + fontSize;
  for (const line of lines) {
    if (line === '') {
      y += lineHeight / 2;
      continue;
    }
    ctx.fillText(line, center, y);
    y += lineHeight;
  }
  ctx.fillStyle = themeColor('--text-subtle');
  ctx.font = `${FOOTER_FONT}px ${font}`;
  const footerTop = height - PADDING - FOOTER_FONT;
  ctx.fillText(poet, center, footerTop);
  ctx.fillText(PROD_DOMAIN, center, footerTop + FOOTER_FONT * LINE_RATIO);
  return canvasBlob(canvas);
}

export function drawVerseCard(card: VerseCardText): ResultAsync<File | null, Error> {
  return ResultAsync.fromSafePromise(document.fonts.ready)
    .andThen(loadLogo)
    .andThen((logo) => drawCard(document.createElement('canvas'), logo, card))
    .map((blob) =>
      blob === null ? null : new File([blob], `${SHARE_TEXTS.imageFile}.png`, { type: 'image/png' })
    );
}
