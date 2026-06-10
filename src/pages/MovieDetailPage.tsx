import { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import { FavoriteToggle } from "../components/favorites/FavoriteToggle";
import {
  ContentCarousel,
  type CarouselItem,
} from "../components/home/ContentCarousel";
import { PlayLogoIcon } from "../components/layout/icons";
import { Button } from "../components/ui/Button";
import { Skeleton } from "../components/ui/Skeleton";
import { CastGrid } from "../components/vod/CastGrid";
import { DetailBackButton } from "../components/vod/DetailBackButton";
import { VodDetailHero } from "../components/vod/VodDetailHero";
import * as api from "../lib/api";
import { getCachedMovie, mergeMovieData, setCachedMovie } from "../lib/detailCache";
import { bestPosterUrl, cardPosterUrl } from "../lib/posterUtils";
import { extractYearFromTitle, formatRating } from "../lib/titleMeta";
import type { Movie } from "../lib/types";

interface MovieDetailPreview {
  id: string;
  name: string;
  poster?: string | null;
  backdrop?: string | null;
  streamUrl?: string;
  profileId?: string;
  category?: string | null;
  plot?: string | null;
  genres?: string | null;
  rating?: string | null;
}

interface MovieDetailLocationState {
  category?: string;
  preview?: MovieDetailPreview;
}

function movieFromPreview(preview: MovieDetailPreview): Movie {
  return {
    id: preview.id,
    profileId: preview.profileId ?? "",
    name: preview.name,
    poster: preview.poster ?? null,
    backdrop: preview.backdrop ?? null,
    plot: preview.plot ?? null,
    genres: preview.genres ?? null,
    rating: preview.rating ?? null,
    streamUrl: preview.streamUrl ?? "",
    category: preview.category ?? null,
    addedAt: 0,
    sortOrder: 0,
  };
}

export function MovieDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  const locationState = location.state as MovieDetailLocationState | null;
  const preview =
    locationState?.preview?.id === id ? locationState?.preview : undefined;
  const [movie, setMovie] = useState<Movie | null>(() =>
    preview ? movieFromPreview(preview) : null,
  );
  const [loading, setLoading] = useState(!preview);
  const [enriching, setEnriching] = useState(Boolean(preview));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    let cancelled = false;

    // The route only swaps the :id param (e.g. clicking a "Semelhantes"
    // card), so the component never remounts — drop the previous movie's
    // state here or its backdrop/cast/plot leak into the new title while
    // it loads.
    setMovie(preview ? movieFromPreview(preview) : null);
    setLoading(!preview);
    setEnriching(true);
    setError(null);

    const cached = getCachedMovie(id);
    if (cached?.streamUrl) {
      // The cache (filled by the hero banner) is richer than the navigation
      // preview — merge it over the preview instead of keeping the preview.
      setMovie((current) =>
        current ? mergeMovieData(current, cached, cached) : cached,
      );
      setLoading(false);
      setEnriching(false);
      if (!cached.similar?.length) {
        // The hero banner pre-caches details without recommendations; fetch
        // just the "Semelhantes" carousel here instead of skipping it.
        void api
          .getMovie(id, { enrich: false, includeSimilar: true })
          .then((withSimilar) => {
            if (cancelled || !withSimilar.similar?.length) return;
            setMovie((current) => {
              const next = current
                ? { ...current, similar: withSimilar.similar }
                : withSimilar;
              setCachedMovie(id!, next);
              return next;
            });
          })
          .catch(() => {
            // Recommendations are optional; keep the cached detail visible.
          });
      }
      return () => {
        cancelled = true;
      };
    }

    async function load() {
      setError(null);
      if (!preview) {
        setLoading(true);
      }
      setEnriching(true);

      try {
        const base = await api.getMovie(id!, {
          enrich: false,
          includeSimilar: false,
        });
        if (cancelled) return;

        setMovie((current) => {
          const merged = mergeMovieData(current, base, base);
          return merged;
        });
        setLoading(false);

        // Enrichment (TMDB) and recommendations are independent — fetch both
        // in parallel instead of serializing the round-trips.
        const enrichedPromise = api.getMovie(id!, {
          enrich: true,
          includeSimilar: false,
        });
        const similarPromise = api.getMovie(id!, {
          enrich: false,
          includeSimilar: true,
        });

        const enriched = await enrichedPromise;
        if (cancelled) return;

        setMovie((current) => {
          const merged = mergeMovieData(current, base, enriched);
          setCachedMovie(id!, merged);
          return merged;
        });
        setEnriching(false);

        const withSimilar = await similarPromise;
        if (!cancelled) {
          setMovie((current) => {
            if (!current) return withSimilar;
            const next = {
              ...current,
              similar: withSimilar.similar ?? current.similar,
            };
            setCachedMovie(id!, next);
            return next;
          });
        }
      } catch (err) {
        if (!cancelled) {
          if (!preview) {
            setError(err instanceof Error ? err.message : String(err));
          }
          setLoading(false);
        }
      } finally {
        if (!cancelled) {
          setEnriching(false);
        }
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, [id, preview]);

  const similarItems = useMemo<CarouselItem[]>(
    () =>
      (movie?.similar ?? []).map((item) => ({
        id: item.id,
        title: item.name,
        posterUrl: bestPosterUrl(item.poster, item.backdrop),
        backdropUrl: item.backdrop ?? null,
      })),
    [movie?.similar],
  );

  const handlePlay = () => {
    if (!movie?.streamUrl) return;
    const category =
      locationState?.category ?? movie.category ?? undefined;
    navigate("/movies", {
      state: {
        vod: {
          name: movie.name,
          streamUrl: movie.streamUrl,
          profileId: movie.profileId,
          itemType: "movie",
          itemId: movie.id,
          poster: movie.poster,
          startFullscreen: true,
        },
        category,
      },
    });
  };

  const handleBack = () => {
    navigate("/movies", {
      state: locationState?.category ? { category: locationState.category } : undefined,
    });
  };

  const handleSimilarClick = (item: CarouselItem) => {
    navigate(`/movies/${item.id}`, {
      state: {
        category: locationState?.category,
        preview: {
          id: item.id,
          name: item.title,
          poster: item.posterUrl ?? null,
          backdrop: item.backdropUrl ?? null,
        },
      },
    });
  };

  if (loading) {
    return (
      <div className="flex h-full flex-col overflow-hidden app-bg">
        <div className="flex-1 overflow-y-auto bg-base-950 px-8 pb-8">
          <Skeleton className="mb-6 h-64 w-full rounded-2xl" />
          <Skeleton className="mb-3 h-8 w-2/3 max-w-md" />
          <Skeleton className="mb-4 h-4 w-48" />
          <Skeleton className="h-24 w-full max-w-2xl" />
        </div>
      </div>
    );
  }

  if (error || !movie) {
    return (
      <div className="flex h-full flex-col overflow-hidden app-bg">
        <div className="flex flex-1 flex-col items-center justify-center gap-4 bg-base-950 text-text-secondary">
          <p>{error ?? "Filme não encontrado"}</p>
          <Button variant="secondary" onClick={() => navigate("/movies")}>
            Voltar aos filmes
          </Button>
        </div>
      </div>
    );
  }

  const category =
    locationState?.category ?? movie.category ?? null;
  const year = extractYearFromTitle(movie.name);
  const rating = formatRating(movie.rating);
  const meta = [category, year, movie.genres, rating].filter(Boolean).join(" • ");
  const heroImage = movie.backdrop;
  const hasCast = Boolean(movie.cast?.length);
  const hasSynopsisSection = Boolean(movie.plot || hasCast || enriching);

  return (
    <div className="flex h-full flex-col overflow-hidden app-bg">
      <div className="scrollbar-thin flex-1 overflow-y-auto bg-base-950">
        <VodDetailHero
          backdrop={heroImage}
          poster={movie.poster}
          backdropPending={enriching}
          backButton={<DetailBackButton onClick={handleBack} />}
        >
          <div className="flex flex-col gap-8 md:flex-row md:items-end">
            {movie.poster && (
              <div className="shrink-0">
                <img
                  src={cardPosterUrl(movie.poster, null) ?? movie.poster}
                  alt={movie.name}
                  decoding="async"
                  referrerPolicy="no-referrer"
                  className="h-56 w-40 rounded-xl object-cover shadow-2xl ring-1 ring-base-700/60 md:h-72 md:w-48"
                />
              </div>
            )}
            <div className="min-w-0 flex-1">
              <h1 className="mb-2 text-4xl font-bold tracking-tight">{movie.name}</h1>
              {meta ? (
                <p className="mb-4 text-sm text-text-secondary">{meta}</p>
              ) : null}
              <div className="flex flex-wrap items-center gap-3">
                <Button
                  onClick={handlePlay}
                  disabled={!movie.streamUrl}
                  className="gap-2"
                >
                  <PlayLogoIcon className="h-4 w-4" />
                  Assistir
                </Button>
                <FavoriteToggle itemType="movie" itemId={movie.id} />
              </div>
            </div>
          </div>
        </VodDetailHero>

        {hasSynopsisSection ? (
          <section className="space-y-6 px-8 py-8">
            {movie.plot ? (
              <div>
                <h2 className="mb-2 text-lg font-semibold">Sinopse</h2>
                <p className="max-w-3xl text-sm leading-relaxed text-text-secondary">
                  {movie.plot}
                </p>
              </div>
            ) : enriching ? (
              <div>
                <h2 className="mb-2 text-lg font-semibold">Sinopse</h2>
                <Skeleton className="h-16 w-full max-w-3xl" />
              </div>
            ) : null}
            {hasCast ? (
              <CastGrid cast={movie.cast ?? []} />
            ) : enriching ? (
              <Skeleton className="h-24 w-full max-w-2xl" />
            ) : null}
          </section>
        ) : null}

        {similarItems.length > 0 ? (
          <div className="px-8 pb-8">
            <ContentCarousel
              title="Semelhantes"
              items={similarItems}
              variant="poster"
              onItemClick={handleSimilarClick}
            />
          </div>
        ) : null}
      </div>
    </div>
  );
}
