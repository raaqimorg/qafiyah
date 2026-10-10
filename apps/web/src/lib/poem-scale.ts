export const POEM_SCALE = {
  font: { initial: 1, min: 0.7, max: 1.5, step: 0.1 },
  spacing: { initial: 1, min: 0.2, max: 1.8, step: 0.2, halvesMin: 0.4 },
} as const;

export function stepScale(
  scale: number,
  direction: 1 | -1,
  range: { readonly min: number; readonly max: number; readonly step: number }
): number {
  const next = Math.min(range.max, Math.max(range.min, scale + direction * range.step));
  return Math.round(next * 10) / 10;
}

export function halvesSpacing(spacingScale: number): number {
  return Math.max(spacingScale, POEM_SCALE.spacing.halvesMin);
}
