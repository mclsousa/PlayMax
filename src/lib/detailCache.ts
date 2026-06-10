import { isTmdbBackdrop } from "./heroUtils";
import type { Movie } from "./types";

const DETAIL_CACHE_TTL_MS = 5 * 60_000;

/** Prefer a TMDB backdrop (final art) over provider URLs, regardless of which
 * fetch round it came from — the DB row must not downgrade the hero preview. */
function pickBackdrop(
  ...candidates: (string | null | undefined)[]
): string | null {
  const tmdb = candidates.find((value) => isTmdbBackdrop(value));
  if (tmdb) return tmdb;
  return candidates.find((value) => value?.trim()) ?? null;
}

interface CacheEntry {
  movie: Movie;
  fetchedAt: number;
}

const movieCache = new Map<string, CacheEntry>();

export function getCachedMovie(id: string): Movie | undefined {
  const entry = movieCache.get(id);
  if (!entry) return undefined;
  if (Date.now() - entry.fetchedAt > DETAIL_CACHE_TTL_MS) {
    movieCache.delete(id);
    return undefined;
  }
  return entry.movie;
}

export function setCachedMovie(id: string, movie: Movie): void {
  movieCache.set(id, { movie, fetchedAt: Date.now() });
}

export function mergeMovieData(
  previous: Movie | null,
  base: Movie,
  enriched: Movie,
): Movie {
  return {
    ...enriched,
    ...base,
    streamUrl: base.streamUrl || enriched.streamUrl,
    profileId: base.profileId || enriched.profileId,
    backdrop: pickBackdrop(enriched.backdrop, base.backdrop, previous?.backdrop),
    poster: enriched.poster ?? base.poster ?? previous?.poster ?? null,
    plot: enriched.plot ?? base.plot ?? previous?.plot ?? null,
    genres: enriched.genres ?? base.genres ?? previous?.genres ?? null,
    rating: enriched.rating ?? base.rating ?? previous?.rating ?? null,
    cast: enriched.cast ?? base.cast ?? previous?.cast ?? null,
    category: base.category ?? enriched.category ?? previous?.category ?? null,
  };
}
