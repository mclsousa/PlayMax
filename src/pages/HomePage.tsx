import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "react-router-dom";
import { HeroBanner } from "../components/home/HeroBanner";
import {
  ContentCarousel,
  type CarouselItem,
} from "../components/home/ContentCarousel";
import { RecentChannelsRow } from "../components/home/RecentChannelsRow";
import { useCatalog } from "../hooks/useCatalog";
import { useContinueWatching } from "../hooks/useContinueWatching";
import { useRecentChannels } from "../hooks/useRecentChannels";
import { useReleaseHero } from "../hooks/useReleaseHero";
import { useActiveProfile } from "../hooks/useActiveProfile";
import { useProfiles } from "../hooks/useProfiles";
import { cardPosterUrl, thumbPosterUrl } from "../lib/posterUtils";
import type { CatalogItem, HistoryEntry, SyncProgress } from "../lib/types";
import { vodPlayPath } from "../lib/vodRoutes";

function toCarouselItem(
  item: CatalogItem,
  options?: { showNovoBadge?: boolean },
): CarouselItem {
  return {
    id: item.id,
    title: item.name,
    posterUrl: cardPosterUrl(item.poster, item.backdrop),
    backdropUrl: item.backdrop ?? null,
    itemType: item.itemType,
    showNovoBadge: options?.showNovoBadge,
  };
}

function toContinueCarouselItem(item: HistoryEntry): CarouselItem {
  const progress =
    item.duration > 0
      ? Math.round((item.position / item.duration) * 100)
      : 0;
  const remainingMin =
    item.duration > 0
      ? Math.max(1, Math.ceil((item.duration - item.position) / 60))
      : null;
  const kindLabel = item.itemType === "movie" ? "Filme" : "Episódio";
  const subtitle =
    remainingMin !== null
      ? `${kindLabel} • ${remainingMin} min rest.`
      : kindLabel;

  return {
    id: item.id,
    itemId: item.itemId,
    title: item.name,
    subtitle,
    progress,
    posterUrl: thumbPosterUrl(item.poster, null),
    itemType: item.itemType,
    streamUrl: item.streamUrl,
    startPosition: item.position,
  };
}

export function HomePage() {
  const navigate = useNavigate();
  const { loading: profilesLoading } = useProfiles();
  const { profileId } = useActiveProfile();
  const {
    recentMovies,
    recentSeries,
    updatedSeries,
    releaseMovies,
    releaseSeries,
    loading: catalogLoading,
    releasesReady,
    refresh,
  } = useCatalog(profileId, profilesLoading);
  const {
    items: continueWatching,
    refresh: refreshContinueWatching,
  } = useContinueWatching(profileId, profilesLoading);
  const { channels: recentChannels } = useRecentChannels(
    profileId,
    profilesLoading,
  );

  const [syncProgress, setSyncProgress] = useState<SyncProgress | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);
  const [showCarousels, setShowCarousels] = useState(false);

  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      setShowCarousels(true);
    });
    return () => cancelAnimationFrame(frame);
  }, []);

  useEffect(() => {
    const unlisten = listen<SyncProgress>("sync-progress", (event) => {
      const payload = event.payload;
      if (payload.profileId === profileId) {
        setSyncProgress(payload.percent >= 100 ? null : payload);
        setSyncError(null);
      }
      if (payload.percent >= 100 && payload.profileId === profileId) {
        void refresh({ force: true });
        void refreshContinueWatching({ force: true });
      }
    });
    // Background syncs (e.g. right after adding a list) fail silently
    // otherwise — surface the error so the user isn't left with an empty
    // catalog and no explanation.
    const unlistenError = listen<string>("sync-error", (event) => {
      setSyncProgress(null);
      setSyncError(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
      void unlistenError.then((fn) => fn());
    };
  }, [profileId, refresh, refreshContinueWatching]);

  const { slides: heroSlides, loading: heroLoading } = useReleaseHero(
    profileId,
    releaseMovies,
    profilesLoading,
    !releasesReady && catalogLoading,
  );
  const hasCatalog =
    recentMovies.length > 0 ||
    recentSeries.length > 0 ||
    updatedSeries.length > 0 ||
    releaseMovies.length > 0 ||
    releaseSeries.length > 0;
  const showCatalogEmpty =
    !profilesLoading && !catalogLoading && !!profileId && !hasCatalog;

  const handleCatalogClick = useCallback(
    (item: CarouselItem) => {
      const path =
        item.itemType === "series"
          ? `/series/${item.id}`
          : `/movies/${item.id}`;
      navigate(path, {
        state: {
          preview: {
            id: item.id,
            name: item.title,
            poster: item.posterUrl ?? null,
            backdrop: item.backdropUrl ?? null,
          },
        },
      });
    },
    [navigate],
  );

  const handleContinueClick = useCallback(
    (item: CarouselItem) => {
      if (!profileId || !item.streamUrl || !item.itemId) return;
      const vod = {
        name: item.title,
        streamUrl: item.streamUrl,
        profileId,
        itemType: item.itemType === "episode" ? "episode" : "movie",
        itemId: item.itemId,
        poster: item.posterUrl,
        startPosition: item.startPosition ?? 0,
      } as const;
      navigate(vodPlayPath(vod), { state: { vod } });
      void refreshContinueWatching();
    },
    [navigate, profileId, refreshContinueWatching],
  );

  const continueItems = useMemo(
    () => continueWatching.map(toContinueCarouselItem),
    [continueWatching],
  );
  const releaseMovieItems = useMemo(
    () => releaseMovies.map((item) => toCarouselItem(item, { showNovoBadge: true })),
    [releaseMovies],
  );
  const recentMovieItems = useMemo(
    () => recentMovies.map((item) => toCarouselItem(item)),
    [recentMovies],
  );
  const releaseSeriesItems = useMemo(
    () => releaseSeries.map((item) => toCarouselItem(item, { showNovoBadge: true })),
    [releaseSeries],
  );
  const recentSeriesItems = useMemo(
    () => recentSeries.map((item) => toCarouselItem(item)),
    [recentSeries],
  );
  const updatedSeriesItems = useMemo(
    () => updatedSeries.map((item) => toCarouselItem(item, { showNovoBadge: true })),
    [updatedSeries],
  );

  return (
    <div className="flex h-full flex-col overflow-hidden app-bg">
      {syncProgress ? (
        <div className="shrink-0 border-b border-base-800 bg-base-900/90 px-8 py-2">
          <div className="mb-1 flex justify-between text-xs text-text-secondary">
            <span>{syncProgress.message}</span>
            <span>{Math.round(syncProgress.percent)}%</span>
          </div>
          <div className="h-1.5 overflow-hidden rounded-full bg-base-800">
            <div
              className="h-full rounded-full bg-accent transition-all duration-300"
              style={{ width: `${syncProgress.percent}%` }}
            />
          </div>
        </div>
      ) : null}
      {syncError ? (
        <div className="flex shrink-0 items-center justify-between gap-4 border-b border-red-500/30 bg-red-500/10 px-8 py-2">
          <p className="text-xs text-red-200">
            Falha ao sincronizar a lista: {syncError}
          </p>
          <button
            type="button"
            onClick={() => setSyncError(null)}
            className="shrink-0 text-xs font-medium text-red-200/80 transition-colors hover:text-red-100"
          >
            Fechar
          </button>
        </div>
      ) : null}
      <div className="scrollbar-thin flex-1 overflow-y-auto bg-base-950 pb-8">
        <HeroBanner
          slides={heroSlides}
          loading={heroLoading}
          hasProfile={!!profileId}
        />
        <div className="min-w-0 space-y-8 px-8 pt-8">
          {showCarousels ? (
            <>
          <RecentChannelsRow channels={recentChannels} />
          {continueItems.length > 0 && (
            <ContentCarousel
              title="Continuar assistindo"
              items={continueItems}
              variant="poster"
              onItemClick={handleContinueClick}
            />
          )}
          {releaseMovieItems.length > 0 && (
            <ContentCarousel
              title="Filmes lançamentos"
              items={releaseMovieItems}
              variant="poster"
              viewAllHref="/movies"
              onItemClick={handleCatalogClick}
            />
          )}
          {recentMovieItems.length > 0 && (
            <ContentCarousel
              title="Filmes adicionados recentemente"
              items={recentMovieItems}
              variant="poster"
              viewAllHref="/movies"
              onItemClick={handleCatalogClick}
            />
          )}
          {releaseSeriesItems.length > 0 && (
            <ContentCarousel
              title="Séries lançamentos"
              items={releaseSeriesItems}
              variant="poster"
              viewAllHref="/series"
              onItemClick={handleCatalogClick}
            />
          )}
          {updatedSeriesItems.length > 0 && (
            <ContentCarousel
              title="Séries atualizadas"
              items={updatedSeriesItems}
              variant="poster"
              viewAllHref="/series"
              onItemClick={handleCatalogClick}
            />
          )}
          {recentSeriesItems.length > 0 && (
            <ContentCarousel
              title="Séries adicionadas recentemente"
              items={recentSeriesItems}
              variant="poster"
              viewAllHref="/series"
              onItemClick={handleCatalogClick}
            />
          )}
            </>
          ) : null}
          {showCatalogEmpty && (
            <section className="rounded-xl border border-base-800 bg-base-900/60 px-6 py-5 text-center">
              <p className="text-sm text-text-secondary">
                Nenhum filme ou série no catálogo ainda.
              </p>
              <p className="mt-1 text-xs text-text-muted">
                Sincronize filmes e séries em Configurações ou use uma lista com
                grupos de VOD.
              </p>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}
