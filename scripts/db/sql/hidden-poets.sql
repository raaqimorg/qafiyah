ALTER TABLE public.poets ADD COLUMN IF NOT EXISTS is_hidden boolean NOT NULL DEFAULT false;
ALTER TABLE public.poems ADD COLUMN IF NOT EXISTS is_hidden boolean NOT NULL DEFAULT false;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'poets_id_is_hidden_key') THEN
    ALTER TABLE public.poets ADD CONSTRAINT poets_id_is_hidden_key UNIQUE (id, is_hidden);
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'poems_poet_id_is_hidden_fkey') THEN
    ALTER TABLE public.poems ADD CONSTRAINT poems_poet_id_is_hidden_fkey
      FOREIGN KEY (poet_id, is_hidden) REFERENCES public.poets (id, is_hidden) ON UPDATE CASCADE;
  END IF;
END $$;
