import { describe, expect, it } from 'vitest';

import { normalizeProxyPath, resolveProxyPath } from './proxy-allowlist';

const resolve = (path: string | undefined, search = '') =>
  resolveProxyPath(path, new URLSearchParams(search));

describe('resolveProxyPath', () => {
  it('allows the endpoints the browser needs', () => {
    expect(resolve('search', 'q=x')).toBe('search');
    expect(resolve('poems/random')).toBe('poems/random');
    expect(resolve('poems', 'poet=imHZ')).toBe('poems');
    expect(resolve('poems/facets', 'poet=imHZ')).toBe('poems/facets');
  });

  it('forwards a poem list only when it names exactly one poet', () => {
    expect(resolve('poems')).toBeUndefined();
    expect(resolve('poems', 'meter=altawil&meter=alkamil')).toBeUndefined();
    expect(resolve('poems', 'poet=imHZ&poet=oNbs')).toBeUndefined();
    expect(resolve('poems/facets', 'meter=altawil')).toBeUndefined();
    expect(resolve('poems/facets', 'poet=imHZ&poet=oNbs')).toBeUndefined();
  });

  it('counts every spelling of poet the api reads', () => {
    expect(resolve('poems', 'poet%5B0%5D=imHZ')).toBe('poems');
    expect(resolve('poems', 'poet=imHZ&poet%5B%5D=oNbs')).toBeUndefined();
    expect(resolve('poems', 'poet%5B0%5D=imHZ&poet%5B1%5D=oNbs')).toBeUndefined();
  });

  it('refuses every other corpus endpoint', () => {
    expect(resolve('poems/slugs')).toBeUndefined();
    expect(resolve('poems/kdmy')).toBeUndefined();
    expect(resolve('poets')).toBeUndefined();
    expect(resolve('poets/slugs')).toBeUndefined();
    expect(resolve('poets/oNbs')).toBeUndefined();
    expect(resolve('meters')).toBeUndefined();
  });

  it('refuses a name every object inherits', () => {
    expect(resolve('constructor')).toBeUndefined();
    expect(resolve('__proto__')).toBeUndefined();
    expect(resolve('toString')).toBeUndefined();
  });

  it('refuses traversal and empty input', () => {
    expect(resolve('../../etc/passwd')).toBeUndefined();
    expect(resolve('poems/random/../../poems')).toBeUndefined();
    expect(resolve('')).toBeUndefined();
    expect(resolve(undefined)).toBeUndefined();
  });

  it('tolerates surrounding slashes', () => {
    expect(resolve('/search')).toBe('search');
    expect(resolve('/poems/random/')).toBe('poems/random');
  });

  it('is case-sensitive so an uppercased endpoint is refused', () => {
    expect(resolve('Search')).toBeUndefined();
    expect(resolve('SEARCH')).toBeUndefined();
  });

  it('refuses a decoded slash that turns into a longer path', () => {
    expect(resolve('search/poems')).toBeUndefined();
  });

  it('refuses a dot segment that escapes the allowlist', () => {
    expect(resolve('search/../poems')).toBeUndefined();
    expect(resolve('poems/random/../search')).toBeUndefined();
  });

  it('refuses an internal double slash', () => {
    expect(resolve('search//x')).toBeUndefined();
  });
});

describe('normalizeProxyPath', () => {
  it('strips leading and trailing slashes only', () => {
    expect(normalizeProxyPath('//search//')).toBe('search');
    expect(normalizeProxyPath(undefined)).toBe('');
  });
});
