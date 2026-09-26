CREATE OR REPLACE FUNCTION public.reattribute_poem(p_poem integer, p_poet integer)
RETURNS void
LANGUAGE plpgsql
SET search_path TO ''
AS $function$
DECLARE
  v_recension_of integer;
  v_era          integer;
BEGIN
  SELECT recension_of_id INTO v_recension_of FROM public.poems WHERE id = p_poem;
  IF NOT FOUND THEN
    RAISE EXCEPTION 'reattribute_poem: poem % does not exist', p_poem;
  END IF;
  IF v_recension_of IS NOT NULL THEN
    RAISE EXCEPTION 'reattribute_poem: % is a recension of %; move its primary', p_poem, v_recension_of;
  END IF;
  SELECT era_id INTO v_era FROM public.poets WHERE id = p_poet;
  IF NOT FOUND THEN
    RAISE EXCEPTION 'reattribute_poem: poet % does not exist', p_poet;
  END IF;
  UPDATE public.poems SET poet_id = p_poet, era_id = v_era
  WHERE id = p_poem OR recension_of_id = p_poem;
END;
$function$;

CREATE OR REPLACE FUNCTION public.merge_poet(p_keep integer, p_absorb integer)
RETURNS void
LANGUAGE plpgsql
SET search_path TO ''
AS $function$
DECLARE
  k         public.poets%ROWTYPE;
  a         public.poets%ROWTYPE;
  v_unknown integer;
BEGIN
  IF p_keep = p_absorb THEN
    RAISE EXCEPTION 'merge_poet: poet % cannot absorb itself', p_keep;
  END IF;
  SELECT * INTO k FROM public.poets WHERE id = p_keep;
  SELECT * INTO a FROM public.poets WHERE id = p_absorb;
  IF k.id IS NULL OR a.id IS NULL THEN
    RAISE EXCEPTION 'merge_poet: both poets must exist (% absorbing %)', p_keep, p_absorb;
  END IF;
  IF k.is_anonymous OR a.is_anonymous THEN
    RAISE EXCEPTION 'merge_poet: % or % is an anonymous poet; move poems with reattribute_poem instead', p_keep, p_absorb;
  END IF;
  IF a.has_avatar AND NOT k.has_avatar THEN
    RAISE EXCEPTION 'merge_poet: % holds the avatar, so it must survive', p_absorb;
  END IF;
  SELECT id INTO v_unknown FROM public.eras WHERE slug = 'ghayrmaruf';
  IF k.era_id <> a.era_id AND k.era_id <> v_unknown AND a.era_id <> v_unknown THEN
    RAISE EXCEPTION 'merge_poet: % and % have different known eras; a person decides', p_keep, p_absorb;
  END IF;
  UPDATE public.poets
  SET era_id = CASE WHEN k.era_id = v_unknown THEN a.era_id ELSE k.era_id END,
      nickname = COALESCE(k.nickname, a.nickname),
      bio = COALESCE(k.bio, a.bio),
      name_en = COALESCE(k.name_en, a.name_en),
      birth_ce = COALESCE(k.birth_ce, a.birth_ce),
      death_ce = COALESCE(k.death_ce, a.death_ce),
      birth_ah = COALESCE(k.birth_ah, a.birth_ah),
      death_ah = COALESCE(k.death_ah, a.death_ah),
      gender_id = COALESCE(k.gender_id, a.gender_id),
      nation_id = COALESCE(k.nation_id, a.nation_id),
      flags = ARRAY(SELECT DISTINCT f FROM unnest(k.flags || a.flags) AS f ORDER BY f)
  WHERE id = p_keep;
  UPDATE public.poems
  SET poet_id = p_keep, era_id = (SELECT era_id FROM public.poets WHERE id = p_keep)
  WHERE poet_id = p_absorb;
  UPDATE public.poet_aliases SET poet_id = p_keep WHERE poet_id = p_absorb;
  DELETE FROM public.poets WHERE id = p_absorb;
  INSERT INTO public.poet_aliases (slug, poet_id) VALUES (a.slug, p_keep);
END;
$function$;
