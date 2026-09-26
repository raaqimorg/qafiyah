CREATE TABLE IF NOT EXISTS public.poet_aliases (
  slug text PRIMARY KEY CHECK (slug ~ '^[A-Za-z]{4}$'),
  poet_id integer NOT NULL REFERENCES public.poets (id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_poet_aliases_poet_id ON public.poet_aliases (poet_id);

CREATE OR REPLACE FUNCTION public.poets_fill_slug()
RETURNS trigger
LANGUAGE plpgsql
SET search_path TO ''
AS $function$
DECLARE
  v_candidate text;
  v_attempt   int;
  v_max       constant int := 10;
BEGIN
  IF NEW.slug IS NOT NULL AND length(btrim(NEW.slug)) > 0 THEN
    RETURN NEW;
  END IF;

  FOR v_attempt IN 1..v_max LOOP
    v_candidate := public.random_poem_slug();
    IF NOT EXISTS (SELECT 1 FROM public.poets WHERE slug = v_candidate)
       AND NOT EXISTS (SELECT 1 FROM public.poet_aliases WHERE slug = v_candidate) THEN
      NEW.slug := v_candidate;
      RETURN NEW;
    END IF;
  END LOOP;

  RAISE EXCEPTION
    'poets_fill_slug: could not find a free slug after % attempts', v_max;
END;
$function$;
