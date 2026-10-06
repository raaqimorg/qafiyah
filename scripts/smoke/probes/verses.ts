import { err, ok, type Result } from 'neverthrow';

import { parseJsonObject } from '../checks';
import { FIXTURE_POET, HALF_LINES_FIXTURE_POEM } from '../fixtures';
import { POEM_DETAIL, WEB } from '../target';

import type { BodyCheck, Probe } from '../types';

function expectEntriesOf(parts: number): BodyCheck {
  return (body): Result<void, string> => {
    const parsed = parseJsonObject(body);
    if (parsed.isErr()) return err(parsed.error);
    const data = parsed.value['data'];
    if (data === null || typeof data !== 'object') return err('response has no data object');
    const { verses, verseCount } = data as Record<string, unknown>;
    if (!Array.isArray(verses)) return err('verses is not an array');
    if (verseCount !== verses.length) {
      return err(`verseCount ${String(verseCount)} differs from ${verses.length} entries`);
    }
    const wrong = verses.findIndex(
      (entry) =>
        !Array.isArray(entry) ||
        entry.length !== parts ||
        entry.some((part) => typeof part !== 'string' || part.length === 0)
    );
    return wrong === -1 ? ok(undefined) : err(`entry ${wrong} is not ${parts} non-empty parts`);
  };
}

function expectPoemLines(halves: boolean): BodyCheck {
  return (body): Result<void, string> => {
    const start = body.indexOf('<article');
    const end = body.indexOf('</article>', start);
    if (start < 0 || end < 0) return err('the page has no poem article');
    const article = body.slice(start, end);
    if (/<p[^>]*>\s*<\/p>/.test(article)) return err('the poem shows an empty line');
    const laidOutAsHalves = article.includes('text-right') && article.includes('text-left');
    if (laidOutAsHalves === halves) return ok(undefined);
    return err(
      halves ? 'verses are not laid out as two halves' : 'single lines are laid out as halves'
    );
  };
}

const [versePoem] = FIXTURE_POET.poems;

export const verseProbes: readonly Probe[] = [
  {
    url: POEM_DETAIL(versePoem),
    expect: 'ok',
    note: 'a poem of full verses: every entry holds two halves, and verseCount counts the entries',
    check: expectEntriesOf(2),
  },
  {
    url: POEM_DETAIL(HALF_LINES_FIXTURE_POEM.slug),
    expect: 'ok',
    note: 'a poem stored as half-lines: every row is its own one-part entry, none paired with the next',
    check: expectEntriesOf(1),
  },
  {
    url: `${WEB}/poems/${versePoem}`,
    expect: 'ok',
    note: 'the page shows full verses as two halves with no empty line',
    check: expectPoemLines(true),
  },
  {
    url: `${WEB}/poems/${HALF_LINES_FIXTURE_POEM.slug}`,
    expect: 'ok',
    note: 'the page shows each half-line on its own, centered, with no empty line',
    check: expectPoemLines(false),
  },
];
