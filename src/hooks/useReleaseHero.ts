import { useEffect, useMemo, useState } from "react";

import { getMovie } from "../lib/api";
import { getCachedMovie, setCachedMovie } from "../lib/detailCache";
import {
  getCachedHeroSlides,
  setCachedHeroSlides,
} from "../lib/heroCache";
import { isQualifiedHeroMovie, looksLikeChannelEntry } from "../lib/heroUtils";
import type { CatalogItem, Movie } from "../lib/types";

export interface HeroSlide {
  id: string;
  name: string;
  backdrop?: string | null;
  poster?: string | null;
  plot?: string | null;
  genres?: string | null;
  rating?: string | null;
  cast?: string | null;
  streamUrl: string;
  profileId: string;
}

const HERO_CANDIDATE_LIMIT = 24;
const HERO_SLIDE_LIMIT = 8;
const ENRICH_CONCURRENCY = 4;

function catalogSlideScore(item: CatalogItem): number {
  let score = 0;
  if (item.backdrop?.trim()) score += 30;
  else if (item.poster?.trim()) score += 5;
  return score + item.addedAt;
}

function movieToHeroSlide(movie: Movie): HeroSlide {
  const cast =
    movie.cast
      ?.map((member) => member.name.trim())
      .filter(Boolean)
      .join(", ") ?? null;

  return {
    id: movie.id,
    name: movie.name,
    backdrop: movie.backdrop,
    poster: movie.poster,
    plot: movie.plot,
    genres: movie.genres,
    rating: movie.rating,
    cast,
    streamUrl: movie.streamUrl,
    profileId: movie.profileId,
  };
}

async function loadQualifiedHeroMovie(id: string): Promise<Movie | null> {
  const cached = getCachedMovie(id);
  if (cached && isQualifiedHeroMovie(cached)) {
    return cached;
  }

  try {
    const movie = await getMovie(id, { enrich: true, includeSimilar: false });
    setCachedMovie(id, movie);
    return isQualifiedHeroMovie(movie) ? movie : null;
  } catch {
    return null;
  }
}

async function buildHeroSlides(
  candidates: CatalogItem[],
  signal: AbortSignal,
  onProgress: (slides: HeroSlide[]) => void,
): Promise<HeroSlide[]> {
  const slides: HeroSlide[] = [];

  for (
    let index = 0;
    index < candidates.length && slides.length < HERO_SLIDE_LIMIT;
    index += ENRICH_CONCURRENCY
  ) {
    if (signal.aborted) break;

    const batch = candidates.slice(index, index + ENRICH_CONCURRENCY);
    const movies = await Promise.all(
      batch.map((item) => loadQualifiedHeroMovie(item.id)),
    );

    let added = false;
    for (const movie of movies) {
      if (!movie || signal.aborted) continue;
      slides.push(movieToHeroSlide(movie));
      added = true;
      if (slides.length >= HERO_SLIDE_LIMIT) break;
    }

    // Surface slides as soon as each batch resolves so the banner appears
    // with the first ready movie instead of waiting for all of them.
    if (added && !signal.aborted) {
      onProgress([...slides]);
    }
  }

  return slides;
}

function pickHeroCandidates(releases: CatalogItem[]): CatalogItem[] {
  return [...releases]
    .filter((item) => item.itemType === "movie")
    .filter((item) => !looksLikeChannelEntry(item))
    .sort((left, right) => catalogSlideScore(right) - catalogSlideScore(left))
    .slice(0, HERO_CANDIDATE_LIMIT);
}

function buildReleaseKey(candidates: CatalogItem[]): string {
  return candidates.map((item) => item.id).join("|");
}

export function useReleaseHero(
  profileId: string | null,
  releases: CatalogItem[],
  profilesLoading: boolean,
  catalogLoading: boolean,
) {
  const candidates = useMemo(
    () => (profileId ? pickHeroCandidates(releases) : []),
    [profileId, releases],
  );
  const releaseKey = useMemo(
    () => buildReleaseKey(candidates),
    [candidates],
  );

  const [slides, setSlides] = useState<HeroSlide[]>(() => {
    if (!profileId || !releaseKey) return [];
    return getCachedHeroSlides(profileId, releaseKey) ?? [];
  });
  const [enriching, setEnriching] = useState(false);

  useEffect(() => {
    if (profilesLoading || !profileId) {
      setSlides([]);
      setEnriching(false);
      return;
    }

    if (candidates.length === 0) {
      setSlides([]);
      setEnriching(false);
      return;
    }

    const cached = getCachedHeroSlides(profileId, releaseKey);
    if (cached) {
      setSlides(cached);
      setEnriching(false);
      return;
    }

    const controller = new AbortController();
    let cancelled = false;

    const run = async () => {
      setEnriching(true);
      const qualified = await buildHeroSlides(candidates, controller.signal, (partial) => {
        if (!cancelled) setSlides(partial);
      });

      if (cancelled || controller.signal.aborted) return;

      setSlides(qualified);
      setCachedHeroSlides(profileId, releaseKey, qualified);
      setEnriching(false);
    };

    void run();

    return () => {
      cancelled = true;
      controller.abort();
    };
  }, [profileId, profilesLoading, releaseKey, candidates]);

  const loading =
    profilesLoading ||
    (!profileId
      ? false
      : enriching && slides.length === 0) ||
    (catalogLoading && candidates.length === 0 && slides.length === 0);

  return { slides, loading };
}
