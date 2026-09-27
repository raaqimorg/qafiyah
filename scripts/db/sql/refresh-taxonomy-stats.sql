DO $$
DECLARE
  stats_rel text;
BEGIN
  FOREACH stats_rel IN ARRAY ARRAY[
    'poet_stats', 'meter_stats', 'rhyme_stats', 'era_stats', 'theme_stats',
    'collection_stats', 'poem_type_stats', 'form_stats', 'register_stats',
    'genre_stats', 'majra_stats', 'nation_stats', 'gender_stats'
  ] LOOP
    IF EXISTS (
      SELECT 1 FROM pg_class c
      JOIN pg_namespace n ON n.oid = c.relnamespace
      WHERE n.nspname = 'public' AND c.relname = stats_rel AND c.relkind = 'v'
    ) THEN
      EXECUTE format('DROP VIEW public.%I', stats_rel);
    END IF;
  END LOOP;
END $$;

CREATE TABLE IF NOT EXISTS public.poet_stats (
  id integer PRIMARY KEY,
  name text,
  slug text,
  era_id integer,
  poems_count bigint
);

CREATE TABLE IF NOT EXISTS public.meter_stats (
  id integer PRIMARY KEY,
  name text,
  slug text,
  poems_count bigint,
  poets_count bigint
);

CREATE TABLE IF NOT EXISTS public.rhyme_stats (
  id integer PRIMARY KEY,
  letter text,
  slug text,
  name text,
  poems_count integer,
  poets_count integer
);

CREATE TABLE IF NOT EXISTS public.era_stats (
  id integer PRIMARY KEY,
  name text,
  slug text,
  sort_order integer,
  poets_count bigint,
  poems_count bigint
);

CREATE TABLE IF NOT EXISTS public.theme_stats (
  id integer PRIMARY KEY,
  name text,
  slug text,
  poems_count bigint,
  poets_count bigint
);

CREATE TABLE IF NOT EXISTS public.collection_stats (
  id integer PRIMARY KEY,
  name text,
  slug text,
  poems_count bigint,
  poets_count bigint
);

CREATE TABLE IF NOT EXISTS public.poem_type_stats (
  id bigint PRIMARY KEY,
  name text,
  slug text,
  poems_count bigint,
  poets_count bigint
);

CREATE TABLE IF NOT EXISTS public.form_stats (
  id bigint PRIMARY KEY,
  name text,
  slug text,
  poems_count bigint,
  poets_count bigint
);

CREATE TABLE IF NOT EXISTS public.register_stats (
  id bigint PRIMARY KEY,
  name text,
  slug text,
  poems_count bigint,
  poets_count bigint
);

CREATE TABLE IF NOT EXISTS public.genre_stats (
  id bigint PRIMARY KEY,
  name text,
  slug text,
  poems_count bigint,
  poets_count bigint
);

CREATE TABLE IF NOT EXISTS public.majra_stats (
  id bigint PRIMARY KEY,
  name text,
  slug text,
  poems_count bigint,
  poets_count bigint
);

CREATE TABLE IF NOT EXISTS public.nation_stats (
  id bigint PRIMARY KEY,
  name text,
  slug text,
  poets_count bigint,
  poems_count bigint
);

CREATE TABLE IF NOT EXISTS public.gender_stats (
  id bigint PRIMARY KEY,
  name text,
  slug text,
  poets_count bigint,
  poems_count bigint
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_poet_stats_slug ON public.poet_stats (slug);
CREATE UNIQUE INDEX IF NOT EXISTS idx_meter_stats_slug ON public.meter_stats (slug);
CREATE UNIQUE INDEX IF NOT EXISTS idx_rhyme_stats_slug ON public.rhyme_stats (slug);
CREATE UNIQUE INDEX IF NOT EXISTS idx_era_stats_slug ON public.era_stats (slug);
CREATE UNIQUE INDEX IF NOT EXISTS idx_theme_stats_slug ON public.theme_stats (slug);
CREATE UNIQUE INDEX IF NOT EXISTS idx_collection_stats_slug ON public.collection_stats (slug);

CREATE OR REPLACE FUNCTION public.refresh_taxonomy_stats()
RETURNS void
LANGUAGE plpgsql
SET search_path TO ''
AS $function$
BEGIN
  DELETE FROM public.poet_stats;
  DELETE FROM public.meter_stats;
  DELETE FROM public.rhyme_stats;
  DELETE FROM public.era_stats;
  DELETE FROM public.theme_stats;
  DELETE FROM public.collection_stats;
  DELETE FROM public.poem_type_stats;
  DELETE FROM public.form_stats;
  DELETE FROM public.register_stats;
  DELETE FROM public.genre_stats;
  DELETE FROM public.majra_stats;
  DELETE FROM public.nation_stats;
  DELETE FROM public.gender_stats;

  INSERT INTO public.poet_stats (id, name, slug, era_id, poems_count)
  SELECT p.id, p.name, p.slug, p.era_id, count(pm.id)
  FROM public.poets p
  LEFT JOIN public.poems pm ON p.id = pm.poet_id AND pm.recension_of_id IS NULL AND NOT pm.is_hidden
  WHERE NOT p.is_hidden
  GROUP BY p.id, p.name, p.slug, p.era_id;

  INSERT INTO public.meter_stats (id, name, slug, poems_count, poets_count)
  SELECT m.id, m.name, m.slug, count(DISTINCT p.id), count(DISTINCT p.poet_id)
  FROM public.meters m
  LEFT JOIN public.poems p ON m.id = p.meter_id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  GROUP BY m.id, m.name, m.slug;

  INSERT INTO public.rhyme_stats (id, letter, slug, name, poems_count, poets_count)
  SELECT r.id, r.letter, r.slug, r.name, count(p.id)::int, count(DISTINCT p.poet_id)::int
  FROM public.rhymes r
  LEFT JOIN public.poems p ON p.rhyme_id = r.id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  GROUP BY r.id, r.letter, r.slug, r.name;

  INSERT INTO public.era_stats (id, name, slug, sort_order, poets_count, poems_count)
  SELECT e.id, e.name, e.slug, e.sort_order,
         COALESCE(poet_counts.count, 0), COALESCE(poem_counts.count, 0)
  FROM public.eras e
  LEFT JOIN (
    SELECT era_id, count(*) AS count FROM public.poets WHERE NOT is_hidden GROUP BY era_id
  ) poet_counts ON e.id = poet_counts.era_id
  LEFT JOIN (
    SELECT p.era_id, count(*) AS count
    FROM public.poems pm
    JOIN public.poets p ON pm.poet_id = p.id
    WHERE pm.recension_of_id IS NULL AND NOT pm.is_hidden
    GROUP BY p.era_id
  ) poem_counts ON e.id = poem_counts.era_id;

  INSERT INTO public.theme_stats (id, name, slug, poems_count, poets_count)
  SELECT t.id, t.name, t.slug, count(DISTINCT p.id), count(DISTINCT pt.id)
  FROM public.themes t
  LEFT JOIN public.poems p ON t.id = p.theme_id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  LEFT JOIN public.poets pt ON p.poet_id = pt.id
  GROUP BY t.id, t.name, t.slug;

  INSERT INTO public.collection_stats (id, name, slug, poems_count, poets_count)
  SELECT c.id, c.name, c.slug, count(DISTINCT p.id), count(DISTINCT pt.id)
  FROM public.collections c
  LEFT JOIN public.poems p ON c.id = p.collection_id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  LEFT JOIN public.poets pt ON p.poet_id = pt.id
  GROUP BY c.id, c.name, c.slug;

  INSERT INTO public.poem_type_stats (id, name, slug, poems_count, poets_count)
  SELECT v.id, v.name, v.slug, count(DISTINCT p.id), count(DISTINCT pt.id)
  FROM public.poem_types v
  LEFT JOIN public.poems p ON v.id = p.poem_type_id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  LEFT JOIN public.poets pt ON p.poet_id = pt.id
  GROUP BY v.id, v.name, v.slug;

  INSERT INTO public.form_stats (id, name, slug, poems_count, poets_count)
  SELECT v.id, v.name, v.slug, count(DISTINCT p.id), count(DISTINCT pt.id)
  FROM public.forms v
  LEFT JOIN public.poems p ON v.id = p.form_id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  LEFT JOIN public.poets pt ON p.poet_id = pt.id
  GROUP BY v.id, v.name, v.slug;

  INSERT INTO public.register_stats (id, name, slug, poems_count, poets_count)
  SELECT v.id, v.name, v.slug, count(DISTINCT p.id), count(DISTINCT pt.id)
  FROM public.registers v
  LEFT JOIN public.poems p ON v.id = p.register_id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  LEFT JOIN public.poets pt ON p.poet_id = pt.id
  GROUP BY v.id, v.name, v.slug;

  INSERT INTO public.genre_stats (id, name, slug, poems_count, poets_count)
  SELECT v.id, v.name, v.slug, count(DISTINCT p.id), count(DISTINCT pt.id)
  FROM public.genres v
  LEFT JOIN public.poems p ON v.id = p.genre_id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  LEFT JOIN public.poets pt ON p.poet_id = pt.id
  GROUP BY v.id, v.name, v.slug;

  INSERT INTO public.majra_stats (id, name, slug, poems_count, poets_count)
  SELECT v.id, v.name, v.slug, count(DISTINCT p.id), count(DISTINCT pt.id)
  FROM public.majras v
  LEFT JOIN public.poems p ON v.id = p.rhyme_majra_id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  LEFT JOIN public.poets pt ON p.poet_id = pt.id
  GROUP BY v.id, v.name, v.slug;

  INSERT INTO public.nation_stats (id, name, slug, poets_count, poems_count)
  SELECT v.id, v.name, v.slug, count(DISTINCT pt.id), count(DISTINCT p.id)
  FROM public.nations v
  LEFT JOIN public.poets pt ON v.id = pt.nation_id AND NOT pt.is_hidden
  LEFT JOIN public.poems p ON p.poet_id = pt.id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  GROUP BY v.id, v.name, v.slug;

  INSERT INTO public.gender_stats (id, name, slug, poets_count, poems_count)
  SELECT v.id, v.name, v.slug, count(DISTINCT pt.id), count(DISTINCT p.id)
  FROM public.genders v
  LEFT JOIN public.poets pt ON v.id = pt.gender_id AND NOT pt.is_hidden
  LEFT JOIN public.poems p ON p.poet_id = pt.id AND p.recension_of_id IS NULL AND NOT p.is_hidden
  GROUP BY v.id, v.name, v.slug;

  ANALYZE public.poet_stats;
  ANALYZE public.meter_stats;
  ANALYZE public.rhyme_stats;
  ANALYZE public.era_stats;
  ANALYZE public.theme_stats;
  ANALYZE public.collection_stats;
END;
$function$;
