const MARK_REGEX = /<mark>([^<]+)<\/mark>/g;
const MARK_SPLIT_REGEX = /<\/?mark>/;
const ADJACENT_MARKS_REGEX = /<\/mark>(\s+)<mark>/g;

export function mergeAdjacentMarks(text: string): string {
  return text.replaceAll(ADJACENT_MARKS_REGEX, '$1');
}

export function highlightedTerms(snippet: string): readonly string[] {
  const merged = mergeAdjacentMarks(snippet);
  const terms: string[] = [];
  for (const match of merged.matchAll(MARK_REGEX)) {
    const term = match[1]?.trim();
    if (term !== undefined && term !== '' && !terms.includes(term)) terms.push(term);
  }
  return terms;
}

export function splitHighlightedParts(text: string): readonly string[] {
  return mergeAdjacentMarks(text).split(MARK_SPLIT_REGEX);
}
