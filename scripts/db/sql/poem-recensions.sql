ALTER TABLE public.poems ADD COLUMN IF NOT EXISTS recension_of_id integer;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'poems_recension_of_id_fkey') THEN
    ALTER TABLE public.poems ADD CONSTRAINT poems_recension_of_id_fkey
      FOREIGN KEY (recension_of_id) REFERENCES public.poems (id);
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'poems_recension_not_self') THEN
    ALTER TABLE public.poems ADD CONSTRAINT poems_recension_not_self CHECK (recension_of_id <> id);
  END IF;
END $$;

DO $$
DECLARE
  stale text;
BEGIN
  FOR stale IN
    SELECT indexname FROM pg_indexes
    WHERE schemaname = 'public' AND tablename = 'poems'
      AND indexname LIKE 'idx\_poems\_primary\_%' AND indexdef NOT LIKE '%is_hidden%'
  LOOP
    EXECUTE format('DROP INDEX public.%I', stale);
  END LOOP;
END $$;

CREATE INDEX IF NOT EXISTS idx_poems_recension_of_id ON public.poems (recension_of_id)
  WHERE recension_of_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_poems_primary_id ON public.poems (id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_poet_id ON public.poems (poet_id, id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_era_id ON public.poems (era_id, id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_meter_id ON public.poems (meter_id, id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_theme_id ON public.poems (theme_id, id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_rhyme_id ON public.poems (rhyme_id, id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_collection_id ON public.poems (collection_id, id)
  WHERE recension_of_id IS NULL AND NOT is_hidden AND collection_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_poems_primary_slug ON public.poems (slug)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_theme_plain ON public.poems (theme_id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_meter_plain ON public.poems (meter_id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;
CREATE INDEX IF NOT EXISTS idx_poems_primary_rhyme_plain ON public.poems (rhyme_id)
  WHERE recension_of_id IS NULL AND NOT is_hidden;

DROP INDEX IF EXISTS public.idx_poems_theme_id;
DROP INDEX IF EXISTS public.idx_poems_meter_id;
DROP INDEX IF EXISTS public.idx_poems_rhyme_id;
DROP INDEX IF EXISTS public.idx_poems_poet_id_ordered;
DROP INDEX IF EXISTS public.idx_poems_era_id_ordered;
DROP INDEX IF EXISTS public.idx_poems_meter_id_ordered;
DROP INDEX IF EXISTS public.idx_poems_theme_id_ordered;
DROP INDEX IF EXISTS public.idx_poems_rhyme_id_ordered;
DROP INDEX IF EXISTS public.idx_poems_collection_id_ordered;
