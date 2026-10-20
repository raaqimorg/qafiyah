export type Tag = readonly [slug: string, strength: number];

const MAX_FEELINGS = 3;
const RATINGS: ReadonlySet<string> = new Set(['1', '2', '3']);

function numberedAnswers(content: string): Readonly<Record<string, unknown>> {
  const parsed: unknown = JSON.parse(content);
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed))
    throw new Error('the reply is not a JSON object');
  return Object.fromEntries(Object.entries(parsed));
}

function asTag(value: unknown, slugs: ReadonlySet<string>): Tag | undefined {
  if (!Array.isArray(value) || value.length !== 2) return undefined;
  const slug: unknown = value[0];
  const rating: unknown = value[1];
  if (typeof slug !== 'string' || !slugs.has(slug)) return undefined;
  if (typeof rating !== 'number' && typeof rating !== 'string') return undefined;
  return RATINGS.has(String(rating)) ? [slug, Number(rating)] : undefined;
}

export function parseClassifyReply(
  content: string,
  count: number,
  slugs: ReadonlySet<string>
): Tag[][] {
  const answers = numberedAnswers(content);
  return Array.from({ length: count }, (_, index) => {
    const listed = answers[String(index + 1)];
    if (!Array.isArray(listed)) throw new Error(`no list of feelings for verse ${index + 1}`);
    return listed
      .map((value: unknown) => asTag(value, slugs))
      .filter((tag) => tag !== undefined)
      .slice(0, MAX_FEELINGS);
  });
}

export function parseVerifyReply(content: string, count: number): boolean[] {
  const answers = numberedAnswers(content);
  return Array.from({ length: count }, (_, index) => {
    const answer = answers[String(index + 1)];
    if (answer !== 'yes' && answer !== 'no') throw new Error(`no yes or no for item ${index + 1}`);
    return answer === 'yes';
  });
}

export function keptTags(tags: readonly Tag[], confirmed: (slug: string) => boolean): Tag[] {
  return tags.filter(([slug, strength]) => strength === 3 || (strength === 2 && confirmed(slug)));
}
