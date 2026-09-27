CREATE TABLE IF NOT EXISTS public.random_poem_pool (
  poet_rank integer NOT NULL,
  poem_id integer NOT NULL REFERENCES public.poems (id) ON DELETE CASCADE,
  PRIMARY KEY (poet_rank, poem_id)
);

CREATE OR REPLACE FUNCTION public.refresh_random_poem_pool()
RETURNS void
LANGUAGE plpgsql
SET search_path TO ''
AS $function$
BEGIN
  TRUNCATE public.random_poem_pool;

  INSERT INTO public.random_poem_pool (poet_rank, poem_id)
  SELECT dense_rank() OVER (ORDER BY p.poet_id), p.id
  FROM public.poems p
  JOIN public.poets      po ON po.id = p.poet_id
  JOIN public.eras       e  ON e.id  = po.era_id
  JOIN public.poem_types ty ON ty.id = p.poem_type_id
  JOIN public.meters     m  ON m.id  = p.meter_id
  WHERE p.recension_of_id IS NULL
    AND NOT p.is_hidden
    AND NOT po.is_anonymous
    AND e.slug IN ('jahili', 'islami', 'umawi', 'abbasi')
    AND ty.slug = 'amudi'
    AND p.verse_count >= 4
    AND m.slug <> 'ghayrmaruf';

  ANALYZE public.random_poem_pool;
END;
$function$;

CREATE OR REPLACE FUNCTION public.random_poem_json()
RETURNS json
LANGUAGE sql
SET search_path TO ''
AS $function$
  WITH pick AS (
    SELECT poem_id
    FROM public.random_poem_pool
    WHERE poet_rank = (
      SELECT floor(random() * max(poet_rank))::int + 1 FROM public.random_poem_pool
    )
    ORDER BY random()
    LIMIT 1
  )
  SELECT json_build_object(
    'poem_id',   p.id,
    'poet_name', pt.name,
    'content',   (
      SELECT string_agg(v.content, '*' ORDER BY pv.position)
      FROM public.poem_verses pv
      JOIN public.verses v ON v.id = pv.verse_id
      WHERE pv.poem_id = p.id
    ),
    'slug',      p.slug
  )
  FROM public.poems p
  JOIN public.poets pt ON pt.id = p.poet_id
  WHERE p.id = (SELECT poem_id FROM pick);
$function$;
