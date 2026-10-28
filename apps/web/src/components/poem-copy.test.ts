import { describe, expect, it } from 'vitest';

import { getPoemSelectionRange } from './poem-copy';

const inside = {};
const outside = {};
const container = { contains: (node: unknown) => node === inside } as unknown as HTMLElement;

function selection({
  collapsed = false,
  ranges = 1,
  start = inside,
  end = inside,
}: {
  readonly collapsed?: boolean;
  readonly ranges?: number;
  readonly start?: object;
  readonly end?: object;
}): Selection {
  const range = { startContainer: start, endContainer: end };
  return {
    isCollapsed: collapsed,
    rangeCount: ranges,
    getRangeAt: () => range,
  } as unknown as Selection;
}

describe('getPoemSelectionRange', () => {
  it('gives the range of a selection inside the poem', () => {
    const inPoem = selection({});
    expect(getPoemSelectionRange(inPoem, container)).toBe(inPoem.getRangeAt(0));
  });

  it('gives nothing when there is no selection', () => {
    expect(getPoemSelectionRange(null, container)).toBeNull();
  });

  it('gives nothing for a collapsed selection', () => {
    expect(getPoemSelectionRange(selection({ collapsed: true }), container)).toBeNull();
  });

  it('gives nothing for a selection of more than one range', () => {
    expect(getPoemSelectionRange(selection({ ranges: 2 }), container)).toBeNull();
  });

  it('gives nothing when the selection starts outside the poem', () => {
    expect(getPoemSelectionRange(selection({ start: outside }), container)).toBeNull();
  });

  it('gives nothing when the selection ends outside the poem', () => {
    expect(getPoemSelectionRange(selection({ end: outside }), container)).toBeNull();
  });
});
