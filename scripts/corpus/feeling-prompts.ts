export type Feeling = {
  readonly slug: string;
  readonly name: string;
  readonly wheelTerm: string;
  readonly description: string;
};

export type Messages = { readonly system: string; readonly user: string };

const CLASSIFY_INSTRUCTIONS = `You classify verses of classical Arabic poetry by the feelings they express.

For each numbered verse, list the feelings from the list below that the poet, or a person in the verse, is clearly feeling in it: up to three, strongest first, and usually one.

Most verses express no feeling: give an empty list for them. That covers description of places, animals, weapons, or battles; narration of events; praise or blame of someone's qualities (courage, generosity, power, beauty); wisdom, sayings, and advice; prayers; and satire, unless the verse shows someone actually feeling something. Boasting counts as pride only when the poet boasts of himself or his people.

Judge the meaning of the whole verse. Classical Arabic words often have several senses: a word that shares letters with a feeling's name is not evidence of that feeling.

Rate each feeling you list: 1 when it is faint or only implied, 2 when it is clear, 3 when it is strong and explicit.

Reply with JSON only, mapping each verse number to a list of [feeling id, rating] pairs, like {"1": [["alhuzn", 3]], "2": []}.

Feelings (id: Arabic name (English): description):`;

const VERIFY_INSTRUCTIONS = `You check feeling labels on verses of classical Arabic poetry.

Each numbered item gives a feeling and a verse. Answer yes only when the poet, or a person in the verse, is clearly feeling that feeling in this verse.

Answer no for praise or blame of someone's qualities (courage, generosity, power, beauty), description of places, nature, animals, or battles, narration of events, wisdom, sayings, advice, prayers, and satire, unless the verse shows someone actually feeling it. Answer no when the verse only uses a word that shares letters with the feeling's name in another sense. Answer no when another feeling fits clearly better.

Reply with JSON only, mapping each item number to "yes" or "no", like {"1": "yes", "2": "no"}.`;

const described = (feeling: Feeling): string =>
  `${feeling.name} (${feeling.wheelTerm}): ${feeling.description}`;

export function classifyMessages(
  feelings: readonly Feeling[],
  verses: readonly string[]
): Messages {
  const list = feelings.map((feeling) => `${feeling.slug}: ${described(feeling)}`).join('\n');
  return {
    system: `${CLASSIFY_INSTRUCTIONS}\n${list}`,
    user: verses.map((verse, index) => `${index + 1}. ${verse}`).join('\n'),
  };
}

export function verifyMessages(
  feelings: readonly Feeling[],
  pairs: readonly (readonly [verse: string, slug: string])[]
): Messages {
  const bySlug = new Map(feelings.map((feeling) => [feeling.slug, feeling]));
  const items = pairs.map(([verse, slug], index) => {
    const feeling = bySlug.get(slug);
    if (feeling === undefined) throw new Error(`unknown feeling: ${slug}`);
    return `${index + 1}. Feeling: ${described(feeling)}\n   Verse: ${verse}`;
  });
  return { system: VERIFY_INSTRUCTIONS, user: items.join('\n') };
}
