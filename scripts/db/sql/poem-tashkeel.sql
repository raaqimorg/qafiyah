CREATE OR REPLACE FUNCTION public.refresh_poem_tashkeel()
RETURNS void
LANGUAGE plpgsql
SET search_path TO ''
SET max_parallel_workers_per_gather TO 0
AS $function$
BEGIN
  UPDATE public.poems p
  SET has_tashkeel = r.vocalized
  FROM (
    SELECT pv.poem_id,
           sum(length(v.content) - length(regexp_replace(v.content, '[ً-ْ]', '', 'g')))
             >= 0.3 * sum(length(regexp_replace(v.content, '[^ء-ي]', '', 'g')))
           AND sum(length(regexp_replace(v.content, '[^ء-ي]', '', 'g'))) > 0 AS vocalized
    FROM public.poem_verses pv
    JOIN public.verses v ON v.id = pv.verse_id
    GROUP BY pv.poem_id
  ) r
  WHERE r.poem_id = p.id
    AND p.has_tashkeel IS DISTINCT FROM r.vocalized;
END;
$function$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public' AND table_name = 'poems' AND column_name = 'has_tashkeel'
  ) THEN
    ALTER TABLE public.poems ADD COLUMN has_tashkeel boolean NOT NULL DEFAULT false;
    PERFORM public.refresh_poem_tashkeel();
  END IF;
END $$;
