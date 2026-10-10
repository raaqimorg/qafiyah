import { describe, expect, test } from 'bun:test';

import { isAtLeast, parseVersion } from './doctor-version';

describe('parseVersion', () => {
  test.each([
    ['GNU bash, version 5.3.9(1)-release (aarch64-apple-darwin25.1.0)', [5, 3, 9]],
    ['GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)', [3, 2, 57]],
    ['git version 2.50.1 (Apple Git-155)', [2, 50, 1]],
    ['Docker version 29.4.0, build 9d7ad9f', [29, 4, 0]],
    ['5.1.2', [5, 1, 2]],
    ['2.40.3-desktop.1', [2, 40, 3]],
    ['v2.24.4', [2, 24, 4]],
    ['rustup 1.29.1 (d95a37b6a 2026-08-13)', [1, 29, 1]],
    ['ShellCheck - shell script analysis tool\nversion: 0.11.0', [0, 11, 0]],
    ['Haskell Dockerfile Linter 2.15.1', [2, 15, 1]],
    ['>=1', [1, 0, 0]],
    ['1.4', [1, 4, 0]],
  ] as const)('reads %p', (text, expected) => {
    expect(parseVersion(text)).toEqual(expected);
  });

  test('finds no version in text without digits', () => {
    expect(parseVersion('command not found')).toBeUndefined();
  });
});

describe('isAtLeast', () => {
  test('compares each part in order', () => {
    expect(isAtLeast([2, 24, 4], [2, 24, 4])).toBe(true);
    expect(isAtLeast([2, 24, 3], [2, 24, 4])).toBe(false);
    expect(isAtLeast([2, 100, 0], [2, 24, 4])).toBe(true);
    expect(isAtLeast([3, 2, 57], [4, 0, 0])).toBe(false);
    expect(isAtLeast([5, 1, 2], [2, 24, 4])).toBe(true);
  });
});
