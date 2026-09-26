BEGIN;

CREATE TEMP TABLE sampled_poem_ids AS
SELECT p.id
FROM poems p
JOIN poets pt ON pt.id = p.poet_id
WHERE pt.slug = ANY (string_to_array(:'fixture_poets', ','));

INSERT INTO sampled_poem_ids
SELECT ranked.id
FROM (
  SELECT first_poem.id, row_number() OVER (ORDER BY first_poem.poet_id) AS position
  FROM (
    SELECT DISTINCT ON (p.poet_id) p.poet_id, p.id
    FROM poems p
    JOIN eras e ON e.id = p.era_id
    JOIN poets pt ON pt.id = p.poet_id
    WHERE e.slug = 'jahili'
      AND NOT pt.slug = ANY (string_to_array(:'fixture_poets', ','))
    ORDER BY p.poet_id, p.id
  ) first_poem
) ranked
WHERE ranked.position <= :poem_count - (SELECT count(*) FROM sampled_poem_ids);

UPDATE poems SET recension_of_id = NULL
WHERE recension_of_id IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM sampled_poem_ids s WHERE s.id = poems.recension_of_id);

DELETE FROM poem_aliases a WHERE NOT EXISTS (
  SELECT 1 FROM sampled_poem_ids s WHERE s.id = a.poem_id
);

DELETE FROM poems p WHERE NOT EXISTS (
  SELECT 1 FROM sampled_poem_ids s WHERE s.id = p.id
);

DELETE FROM verses v WHERE NOT EXISTS (
  SELECT 1 FROM poem_verses pv WHERE pv.verse_id = v.id
);

DELETE FROM poet_aliases a WHERE NOT EXISTS (
  SELECT 1 FROM poems p WHERE p.poet_id = a.poet_id
);

DELETE FROM poets pt WHERE NOT EXISTS (
  SELECT 1 FROM poems p WHERE p.poet_id = pt.id
);

COMMIT;
