export function getPoemSelectionRange(
  selection: Selection | null,
  container: HTMLElement
): Range | null {
  if (selection === null || selection.isCollapsed || selection.rangeCount !== 1) return null;
  const range = selection.getRangeAt(0);
  return container.contains(range.startContainer) && container.contains(range.endContainer)
    ? range
    : null;
}

export function rangeInside(range: Range, node: Node): Range {
  const part = document.createRange();
  part.selectNodeContents(node);
  if (range.compareBoundaryPoints(Range.START_TO_START, part) > 0) {
    part.setStart(range.startContainer, range.startOffset);
  }
  if (range.compareBoundaryPoints(Range.END_TO_END, part) < 0) {
    part.setEnd(range.endContainer, range.endOffset);
  }
  return part;
}

function selectedHemistichs(range: Range, verse: Element): string {
  return [...verse.querySelectorAll('p')]
    .filter((line) => range.intersectsNode(line))
    .map((line) => rangeInside(range, line).toString().trim())
    .filter((line) => line !== '')
    .join('\n');
}

export function selectedVerses(range: Range, column: HTMLElement): string {
  return [...column.children]
    .map((verse) => selectedHemistichs(range, verse))
    .filter((verse) => verse !== '')
    .join('\n\n');
}

export function lastSelectedVerse(range: Range, column: HTMLElement): Element | null {
  for (let index = column.children.length - 1; index >= 0; index -= 1) {
    const verse = column.children.item(index);
    if (verse !== null && selectedHemistichs(range, verse) !== '') return verse;
  }
  return null;
}
