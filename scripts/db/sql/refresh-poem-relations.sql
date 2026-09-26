CREATE OR REPLACE FUNCTION public.refresh_poem_relations()
RETURNS void
LANGUAGE plpgsql
SET search_path TO ''
SET work_mem TO '64MB'
AS $function$
BEGIN
  CREATE TEMP TABLE tmp_base ON COMMIT DROP AS
  SELECT p.id, p.poet_id, p.era_id, p.theme_id, p.meter_id, p.rhyme_id, p.poem_type_id, p.recension_of_id
  FROM public.poems p;
  CREATE INDEX ON tmp_base (id);

  CREATE TEMP TABLE tmp_pool ON COMMIT DROP AS
  SELECT b.id, b.poet_id, b.era_id, b.theme_id, b.meter_id, b.rhyme_id
  FROM tmp_base b
  JOIN public.poets      pt ON pt.id = b.poet_id
  JOIN public.eras       e  ON e.id  = b.era_id
  JOIN public.meters     m  ON m.id  = b.meter_id
  JOIN public.poem_types ty ON ty.id = b.poem_type_id
  WHERE NOT pt.is_anonymous
    AND e.slug IN ('jahili', 'islami', 'umawi', 'abbasi', 'andalusi', 'fatimi', 'ayyubi', 'mamluki')
    AND m.slug <> 'ghayrmaruf'
    AND ty.slug = 'amudi'
    AND b.recension_of_id IS NULL;
  CREATE INDEX ON tmp_pool (id);

  CREATE TEMP TABLE tmp_ranked ON COMMIT DROP AS
  SELECT 'poet'::text AS cat, poet_id AS part, id,
         (row_number() OVER (PARTITION BY poet_id  ORDER BY hashtextextended(id::text, 11)))::int AS rn
  FROM tmp_pool
  UNION ALL
  SELECT 'era', era_id, id,
         (row_number() OVER (PARTITION BY era_id   ORDER BY hashtextextended(id::text, 12)))::int
  FROM tmp_pool
  UNION ALL
  SELECT 'theme', theme_id, id,
         (row_number() OVER (PARTITION BY theme_id ORDER BY hashtextextended(id::text, 13)))::int
  FROM tmp_pool
  UNION ALL
  SELECT 'meter', meter_id, id,
         (row_number() OVER (PARTITION BY meter_id ORDER BY hashtextextended(id::text, 14)))::int
  FROM tmp_pool
  UNION ALL
  SELECT 'rhyme', rhyme_id, id,
         (row_number() OVER (PARTITION BY rhyme_id ORDER BY hashtextextended(id::text, 15)))::int
  FROM tmp_pool;
  CREATE INDEX ON tmp_ranked (cat, part, rn);

  CREATE TEMP TABLE tmp_sizes ON COMMIT DROP AS
  SELECT cat, part, count(*)::int AS cnt
  FROM tmp_ranked
  GROUP BY cat, part;
  CREATE INDEX ON tmp_sizes (cat, part);

  ANALYZE tmp_base;
  ANALYZE tmp_pool;
  ANALYZE tmp_ranked;
  ANALYZE tmp_sizes;

  CREATE TEMP TABLE tmp_relations ON COMMIT DROP AS
  WITH
  targets AS (
    SELECT
      b.id AS poem_id,
      b.recension_of_id AS own_primary,
      x.cat,
      x.part,
      g.k,
      (((hashtextextended(b.id::text || x.cat, 7) & 9223372036854775807) + g.k) % s.cnt) + 1 AS rn_target
    FROM tmp_base b
    CROSS JOIN LATERAL (VALUES
      ('poet'::text,  b.poet_id,   4),
      ('era',         b.era_id,    16),
      ('theme',       b.theme_id,  4),
      ('meter',       b.meter_id,  4),
      ('rhyme',       b.rhyme_id,  4)
    ) AS x(cat, part, m)
    JOIN tmp_sizes s ON s.cat = x.cat AND s.part = x.part
    CROSS JOIN LATERAL generate_series(0, x.m - 1) AS g(k)
  ),

  hits AS (
    SELECT t.poem_id, r.id AS related_id, t.cat, min(t.k) AS k
    FROM targets t
    JOIN tmp_ranked r ON r.cat = t.cat AND r.part = t.part AND r.rn = t.rn_target
    WHERE r.id <> t.poem_id AND r.id IS DISTINCT FROM t.own_primary
    GROUP BY t.poem_id, r.id, t.cat
  ),

  home AS (
    SELECT DISTINCT ON (poem_id, related_id)
      poem_id, related_id, cat, k,
      CASE cat WHEN 'poet' THEN 1 WHEN 'era' THEN 2 WHEN 'theme' THEN 3
               WHEN 'meter' THEN 4 ELSE 5 END AS pri
    FROM hits
    ORDER BY poem_id, related_id,
             CASE cat WHEN 'poet' THEN 1 WHEN 'era' THEN 2 WHEN 'theme' THEN 3
                      WHEN 'meter' THEN 4 ELSE 5 END,
             k
  ),

  capped AS (
    SELECT *,
      row_number() OVER (PARTITION BY poem_id, cat ORDER BY k) AS cat_rn
    FROM home
  ),

  pool AS (
    SELECT poem_id, related_id,
      CASE WHEN cat = 'era' AND cat_rn > 2 THEN 6 ELSE pri END AS grp,
      cat_rn AS ord
    FROM capped
    WHERE cat = 'era' OR cat_rn <= 2
  ),

  ranked_final AS (
    SELECT poem_id, related_id, grp,
      row_number() OVER (PARTITION BY poem_id ORDER BY grp, ord, related_id)::smallint AS rank
    FROM pool
  )

  SELECT poem_id, related_id, (6 - grp)::smallint AS score, rank
  FROM ranked_final
  WHERE rank <= 10;

  ALTER TABLE public.poem_relations
    DROP CONSTRAINT poem_relations_poem_id_fkey,
    DROP CONSTRAINT poem_relations_related_id_fkey;

  TRUNCATE public.poem_relations;

  INSERT INTO public.poem_relations (poem_id, related_id, score, rank)
  SELECT poem_id, related_id, score, rank
  FROM tmp_relations;

  ALTER TABLE public.poem_relations
    ADD CONSTRAINT poem_relations_poem_id_fkey
      FOREIGN KEY (poem_id) REFERENCES public.poems (id) ON DELETE CASCADE,
    ADD CONSTRAINT poem_relations_related_id_fkey
      FOREIGN KEY (related_id) REFERENCES public.poems (id) ON DELETE CASCADE;
END;
$function$;
