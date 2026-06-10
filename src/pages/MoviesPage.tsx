import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useLocation, useNavigate } from "react-router-dom";

import { CatalogCategorySearch } from "../components/categories/CatalogCategorySearch";
import { CatalogSortPills } from "../components/categories/CatalogSortPills";
import { CategoryContentHeader } from "../components/categories/CategoryContentHeader";

import { CategoryCards, formatGroupLabel } from "../components/live/CategoryCards";

import { PlayerOverlay } from "../components/player/PlayerOverlay";
import { Skeleton } from "../components/ui/Skeleton";

import { VodList } from "../components/vod/VodList";

import { useDebouncedValue } from "../hooks/useDebouncedValue";
import { useGuardedCategories } from "../hooks/useGuardedCategories";

import { useMovies } from "../hooks/useMovies";
import { usePlayerActions, usePlayerSession } from "../contexts/PlayerContext";
import type { CatalogSort } from "../lib/api";

import { useActiveProfile } from "../hooks/useActiveProfile";
import type { Movie, VodPlayInfo } from "../lib/types";

interface MoviesLocationState {
  vod?: VodPlayInfo;
  category?: string;
}

function activeMovieIdFromPlayer(
  mediaKind: "live" | "vod" | null,
  playingChannelId: string | null | undefined,
): string | null {
  if (mediaKind !== "vod" || !playingChannelId?.startsWith("vod-")) return null;
  return playingChannelId.slice(4);
}

export function MoviesPage() {
  const location = useLocation();
  const navigate = useNavigate();
  const playedVodRef = useRef<string | null>(null);

  const { activeProfile, profileId } = useActiveProfile();

  const locationState = location.state as MoviesLocationState | null;

  const [category, setCategory] = useState<string | null>(
    locationState?.category ?? null,
  );

  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<CatalogSort>("recent");

  const debouncedSearch = useDebouncedValue(search, 300);

  const session = usePlayerSession();
  const { playVod } = usePlayerActions();

  const { movies, total, categories, categoryCounts, loading, categoriesLoading, loadMore } = useMovies(
    profileId,
    category,
    debouncedSearch,
    sort,
  );

  useEffect(() => {
    if (locationState?.category) {
      setCategory(locationState.category);
    }
  }, [locationState?.category]);

  const vodState = locationState?.vod;

  useEffect(() => {
    if (!vodState) return;

    const key = `${vodState.itemId}:${vodState.streamUrl}`;

    if (playedVodRef.current === key) return;

    playedVodRef.current = key;

    const nextCategory = locationState?.category ?? "all";
    setCategory(nextCategory);

    playVod(vodState);

    navigate("/movies", { replace: true, state: null });
  }, [vodState, locationState?.category, playVod, navigate]);

  const { lockedIds, guardSelect } = useGuardedCategories(categories);

  const handleCategorySelect = (categoryId: string) => {
    guardSelect(categoryId, (nextCategory) => {
      setCategory(nextCategory);
      setSearch("");
    });
  };

  const handleOpenMovie = useCallback((movie: Movie) => {
    navigate(`/movies/${movie.id}`, {
      state: {
        category: category ?? movie.category ?? undefined,
        preview: {
          id: movie.id,
          name: movie.name,
          poster: movie.poster ?? null,
          backdrop: movie.backdrop ?? null,
          streamUrl: movie.streamUrl,
          profileId: movie.profileId,
          category: movie.category ?? null,
        },
      },
    });
  }, [category, navigate]);

  const showItems = category !== null;

  const activeMovieId = activeMovieIdFromPlayer(
    session.mediaKind,
    session.playingChannel?.id,
  );

  const moviesById = useMemo(
    () => new Map(movies.map((movie) => [movie.id, movie])),
    [movies],
  );

  const vodListItems = useMemo(
    () =>
      movies.map((movie) => ({
        id: movie.id,
        title: movie.name?.trim() || movie.id,
        poster: movie.poster,
        backdrop: movie.backdrop,
      })),
    [movies],
  );

  const handleItemClick = useCallback(
    (item: { id: string }) => {
      const movie = moviesById.get(item.id);
      if (movie) handleOpenMovie(movie);
    },
    [moviesById, handleOpenMovie],
  );

  const searchPlaceholder = showItems
    ? `Buscar em ${formatGroupLabel(category!)}`
    : "Selecione uma categoria para buscar";

  return (
    <div className="player-route flex min-h-0 flex-1 flex-col overflow-hidden">
      <CategoryCards
        groups={categories}
        counts={categoryCounts}
        onSelect={handleCategorySelect}
        orientation="horizontal"
        selectedId={showItems ? category : null}
        loading={categoriesLoading}
        lockedIds={lockedIds}
      />

      <div className="flex min-h-0 flex-1 overflow-hidden">
        <div className="channel-panel flex min-h-0 w-full max-w-md shrink-0 flex-col overflow-hidden border-r border-base-800/80 bg-base-950">
          <div className="shrink-0 border-b border-base-800/80 px-4 py-2.5">
            <CatalogCategorySearch
              value={search}
              onChange={setSearch}
              disabled={!showItems}
              placeholder={searchPlaceholder}
            />
          </div>

          {showItems ? (
            <>
            <CategoryContentHeader
              categoryLabel={formatGroupLabel(category)}
              subtitle={`${activeProfile?.name ?? "Nenhuma lista"} · ${total.toLocaleString()} filmes`}
              actions={<CatalogSortPills value={sort} onChange={setSort} />}
              icon={
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.75"
                  className="h-4 w-4 text-text-secondary"
                >
                  <rect x="2" y="4" width="20" height="16" rx="2" />
                  <path d="M8 10h8M8 14h5" strokeLinecap="round" />
                </svg>
              }
            />

            <VodList
              items={vodListItems}
              total={total}
              loading={loading}
              activeItemId={activeMovieId}
              emptyMessage={
                debouncedSearch.trim()
                  ? `Nenhum filme encontrado para "${debouncedSearch.trim()}"`
                  : category === "all"
                    ? "Nenhum filme encontrado"
                    : `Nenhum filme em "${category}"`
              }
              onItemClick={handleItemClick}
              onLoadMore={() => void loadMore()}
            />
            </>
          ) : categoriesLoading ? (
          <div className="flex min-h-0 flex-1 flex-col gap-3 px-4 py-4">
            <Skeleton className="h-14 w-full rounded-xl" />
            {Array.from({ length: 6 }).map((_, index) => (
              <Skeleton key={index} className="h-12 w-full rounded-lg" />
            ))}
          </div>
        ) : (
          <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-8 py-16 text-center">
            <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-base-800/80 ring-1 ring-base-700/50">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                className="h-7 w-7 text-text-muted"
              >
                <rect x="2" y="4" width="20" height="16" rx="2" />
                <path d="M8 10h8M8 14h5" strokeLinecap="round" />
              </svg>
            </div>
            <p className="text-sm text-text-secondary">Selecione uma categoria</p>
            <p className="max-w-xs text-xs text-text-muted">
              Escolha Lançamentos, Ação, Comédia ou outra categoria para ver os filmes
            </p>
          </div>
        )}
        </div>

        <div className="player-video-column flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
          <PlayerOverlay />
        </div>
      </div>
    </div>
  );
}
