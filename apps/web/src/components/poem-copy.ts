import { formatPoemCopySignature } from '@/lib/constants/copy';
import { MAX_HIGHLIGHT_TERM_LENGTH, MAX_HIGHLIGHT_TERMS } from '@/lib/highlight';
import { poemHighlightUrl } from '@/lib/urls';

export type PoemCopy = {
  readonly text: string;
  readonly url: string;
};

function splitHighlightLine(text: string): readonly string[] {
  const terms: string[] = [];
  let phrase = '';
  for (const word of text.trim().match(/\S+\s*/gu) ?? []) {
    if (word.trimEnd().length > MAX_HIGHLIGHT_TERM_LENGTH) {
      if (phrase !== '') terms.push(phrase.trimEnd());
      phrase = '';
      continue;
    }
    const next = phrase + word;
    if (next.trimEnd().length > MAX_HIGHLIGHT_TERM_LENGTH) {
      terms.push(phrase.trimEnd());
      phrase = word;
    } else {
      phrase = next;
    }
  }
  if (phrase !== '') terms.push(phrase.trimEnd());
  return terms;
}

export function poemSelectionUrl(canonicalUrl: string, lines: readonly string[]): string {
  const highlights = [...new Set(lines.flatMap((line) => splitHighlightLine(line)))].slice(
    0,
    MAX_HIGHLIGHT_TERMS
  );
  return poemHighlightUrl(canonicalUrl, highlights);
}

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

export function getPoemCopy(
  selection: Selection | null,
  container: HTMLElement,
  poetName: string,
  canonicalUrl: string
): PoemCopy | null {
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
          return {
            text,
            source: line.dataset['copyText'] ?? line.textContent.trim(),
            isFullLine: text !== '' && text === line.textContent.trim(),
          };
        })
        .filter((line) => line.text !== '')
    )
    .filter((verse) => verse.length > 0);
  if (!selectedVerses.some((verse) => verse.some((line) => line.isFullLine))) return null;
  const text = selectedVerses
    .map((verse) => verse.map((line) => line.text).join('\n'))
    .join('\n\n');
  const url = poemSelectionUrl(
    canonicalUrl,
    selectedVerses.flatMap((verse) =>
      verse.filter((line) => line.isFullLine).map((line) => line.source)
    )
  );
  return { text: `${text}\n\n${formatPoemCopySignature(poetName, url)}`, url };
}
