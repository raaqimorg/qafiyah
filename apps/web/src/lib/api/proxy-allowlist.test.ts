import { describe, expect, it } from 'vitest';

import { normalizeProxyPath, resolveProxyPath } from './proxy-allowlist';

describe('resolveProxyPath', () => {
  it('allows the endpoints the browser needs', () => {
    expect(resolveProxyPath('search')).toBe('search');
    expect(resolveProxyPath('poems/random')).toBe('poems/random');
    expect(resolveProxyPath('poems')).toBe('poems');
    expect(resolveProxyPath('poems/facets')).toBe('poems/facets');
  });

  it('refuses every other corpus endpoint', () => {
    expect(resolveProxyPath('poems/slugs')).toBeUndefined();
    expect(resolveProxyPath('poems/kdmy')).toBeUndefined();
    expect(resolveProxyPath('poets')).toBeUndefined();
    expect(resolveProxyPath('poets/slugs')).toBeUndefined();
    expect(resolveProxyPath('poets/oNbs')).toBeUndefined();
    expect(resolveProxyPath('meters')).toBeUndefined();
  });

  it('refuses traversal and empty input', () => {
    expect(resolveProxyPath('../../etc/passwd')).toBeUndefined();
    expect(resolveProxyPath('poems/random/../../poems')).toBeUndefined();
    expect(resolveProxyPath('')).toBeUndefined();
    expect(resolveProxyPath(undefined)).toBeUndefined();
  });

  it('tolerates surrounding slashes', () => {
    expect(resolveProxyPath('/search')).toBe('search');
    expect(resolveProxyPath('/poems/random/')).toBe('poems/random');
  });

  it('is case-sensitive so an uppercased endpoint is refused', () => {
    expect(resolveProxyPath('Search')).toBeUndefined();
    expect(resolveProxyPath('SEARCH')).toBeUndefined();
  });

  it('refuses a decoded slash that turns into a longer path', () => {
    expect(resolveProxyPath('search/poems')).toBeUndefined();
  });

  it('refuses a dot segment that escapes the allowlist', () => {
    expect(resolveProxyPath('search/../poems')).toBeUndefined();
    expect(resolveProxyPath('poems/random/../search')).toBeUndefined();
  });

  it('refuses an internal double slash', () => {
    expect(resolveProxyPath('search//x')).toBeUndefined();
  });
});

describe('normalizeProxyPath', () => {
  it('strips leading and trailing slashes only', () => {
    expect(normalizeProxyPath('//search//')).toBe('search');
    expect(normalizeProxyPath(undefined)).toBe('');
  });
});
