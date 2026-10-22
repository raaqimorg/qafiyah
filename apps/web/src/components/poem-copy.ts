export function getPoemSelectionText(
  selection: Selection | null,
  container: HTMLElement
): string | null {
  if (selection === null || selection.isCollapsed || selection.rangeCount !== 1) return null;
  const range = selection.getRangeAt(0);
  if (!container.contains(range.startContainer) || !container.contains(range.endContainer)) {
    return null;
  }
  const text = selection.toString().trim();
  return text === '' ? null : text;
}

function selectedTextIn(range: Range, node: Node): string {
  const part = document.createRange();
  part.selectNodeContents(node);
  if (range.compareBoundaryPoints(Range.START_TO_START, part) > 0) {
    part.setStart(range.startContainer, range.startOffset);
  }
  if (range.compareBoundaryPoints(Range.END_TO_END, part) < 0) {
    part.setEnd(range.endContainer, range.endOffset);
  }
  return part.toString();
}

export function lastSelectedRow(range: Range, rows: HTMLCollection): number | null {
  for (let index = rows.length - 1; index >= 0; index -= 1) {
    const row = rows.item(index);
    if (row !== null && selectedTextIn(range, row).trim() !== '') return index;
  }
  return null;
}
