function getSelectedLineText(range: Range, line: HTMLParagraphElement): string {
  if (!range.intersectsNode(line)) return '';

  const lineRange = line.ownerDocument.createRange();
  lineRange.selectNodeContents(line);
  const overlap = range.cloneRange();
  if (overlap.compareBoundaryPoints(Range.START_TO_START, lineRange) < 0) {
    overlap.setStart(line, 0);
  }
  if (overlap.compareBoundaryPoints(Range.END_TO_END, lineRange) > 0) {
    overlap.setEnd(line, line.childNodes.length);
  }
  return overlap.toString().trim();
}

export function getPoemCopyText(
  selection: Selection | null,
  container: HTMLElement,
  signature: string
): string | null {
  if (selection === null || selection.isCollapsed || selection.rangeCount !== 1) return null;
  const range = selection.getRangeAt(0);
  if (!container.contains(range.startContainer) || !container.contains(range.endContainer)) {
    return null;
  }
  const selectedVerses = [...container.querySelectorAll('[data-verse]')]
    .map((verse) =>
      [...verse.querySelectorAll('p')]
        .map((line) => {
          const text = getSelectedLineText(range, line);
          return { text, isFullLine: text !== '' && text === line.textContent.trim() };
        })
        .filter((line) => line.text !== '')
    )
    .filter((verse) => verse.length > 0);
  if (!selectedVerses.some((verse) => verse.some((line) => line.isFullLine))) return null;
  const text = selectedVerses
    .map((verse) => verse.map((line) => line.text).join('\n'))
    .join('\n\n');
  return `${text}\n\n${signature}`;
}
