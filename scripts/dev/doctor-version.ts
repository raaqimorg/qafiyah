export type Version = readonly [number, number, number];

const VERSION = /(\d+)(?:\.(\d+))?(?:\.(\d+))?/;

export function parseVersion(text: string): Version | undefined {
  const match = VERSION.exec(text);
  if (match === null) return undefined;
  const [, major = '0', minor = '0', patch = '0'] = match;
  return [Number(major), Number(minor), Number(patch)];
}

export function isAtLeast(found: Version, minimum: Version): boolean {
  const firstDifference = found
    .map((part, index) => part - (minimum[index] ?? 0))
    .find((difference) => difference !== 0);
  return firstDifference === undefined || firstDifference > 0;
}
