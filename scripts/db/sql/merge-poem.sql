CREATE OR REPLACE FUNCTION public.fill_poem_gaps(p_keep integer, p_source integer)
RETURNS void
LANGUAGE plpgsql
SET search_path TO ''
AS $function$
BEGIN
  UPDATE public.poems k
  SET meter_id = CASE WHEN km.slug = 'ghayrmaruf' THEN s.meter_id ELSE k.meter_id END,
      theme_id = CASE WHEN kt.slug = 'almutafarriqat' THEN s.theme_id ELSE k.theme_id END,
      collection_id = COALESCE(k.collection_id, s.collection_id),
      genre_id = COALESCE(k.genre_id, s.genre_id),
      flags = ARRAY(SELECT DISTINCT f FROM unnest(k.flags || s.flags) AS f ORDER BY f)
  FROM public.poems s, public.meters km, public.themes kt
  WHERE k.id = p_keep
    AND s.id = p_source
    AND km.id = k.meter_id
    AND kt.id = k.theme_id;
END;
$function$;

CREATE OR REPLACE FUNCTION public.merge_poem(p_keep integer, p_absorb integer)
RETURNS void
LANGUAGE plpgsql
SET search_path TO ''
AS $function$
DECLARE
  v_keep_poet        integer;
  v_keep_recension   integer;
  v_keep_collection  integer;
  v_absorb_poet      integer;
  v_absorb_slug      text;
  v_absorb_recension integer;
  v_absorb_collection integer;
BEGIN
  IF p_keep = p_absorb THEN
    RAISE EXCEPTION 'merge_poem: poem % cannot absorb itself', p_keep;
  END IF;
  SELECT poet_id, recension_of_id, collection_id
    INTO v_keep_poet, v_keep_recension, v_keep_collection
    FROM public.poems WHERE id = p_keep;
  SELECT poet_id, slug, recension_of_id, collection_id
    INTO v_absorb_poet, v_absorb_slug, v_absorb_recension, v_absorb_collection
    FROM public.poems WHERE id = p_absorb;
  IF v_keep_poet IS NULL OR v_absorb_poet IS NULL THEN
    RAISE EXCEPTION 'merge_poem: both poems must exist (% absorbing %)', p_keep, p_absorb;
  END IF;
  IF v_keep_poet <> v_absorb_poet THEN
    RAISE EXCEPTION 'merge_poem: % and % belong to different poets, which is an attribution question', p_keep, p_absorb;
  END IF;
  IF v_absorb_collection IS NOT NULL AND v_keep_collection IS NULL THEN
    RAISE EXCEPTION 'merge_poem: % is in a collection and must survive, not be absorbed by %', p_absorb, p_keep;
  END IF;
  IF (v_keep_recension IS NOT NULL AND v_keep_recension <> p_absorb)
     OR (v_absorb_recension IS NOT NULL AND v_absorb_recension <> p_keep) THEN
    RAISE EXCEPTION 'merge_poem: % and % involve a recension of a third poem; a person decides', p_keep, p_absorb;
  END IF;
  PERFORM public.fill_poem_gaps(p_keep, p_absorb);
  UPDATE public.poem_aliases SET poem_id = p_keep WHERE poem_id = p_absorb;
  UPDATE public.poems
  SET recension_of_id = CASE WHEN id = p_keep THEN NULL ELSE p_keep END
  WHERE recension_of_id = p_absorb;
  DELETE FROM public.poems WHERE id = p_absorb;
  INSERT INTO public.poem_aliases (slug, poem_id) VALUES (v_absorb_slug, p_keep);
END;
$function$;
