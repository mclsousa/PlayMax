import { useEffect, useRef, useState } from "react";

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

import { useActiveProfile } from "../hooks/useActiveProfile";

import { usePlayerActions, usePlayerSession } from "../contexts/PlayerContext";
import { useSeries } from "../hooks/useSeries";
import * as api from "../lib/api";
import { buildEpisodeNavItems } from "../lib/playerNavigation";
import type { CatalogSort } from "../lib/api";
import type { Episode, Series, VodPlayInfo } from "../lib/types";

interface SeriesLocationState {
  vod?: VodPlayInfo;
  category?: string;
  seriesId?: string;
}

export function SeriesPage() {
  const location = useLocation();
  const navigate = useNavigate();
  const playedVodRef = useRef<string | null>(null);

  const { activeProfile, profileId } = useActiveProfile();

  const locationState = location.state as SeriesLocationState | null;

  const [category, setCategory] = useState<string | null>(
    locationState?.category ?? null,
  );

  const [playingSeriesId, setPlayingSeriesId] = useState<string | null>(
    locationState?.seriesId ?? null,
  );

  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<CatalogSort>("recent");
  const [browseSeriesCatalog, setBrowseSeriesCatalog] = useState(false);

  const debouncedSearch = useDebouncedValue(search, 300);

  const session = usePlayerSession();
  const { playVod } = usePlayerActions();

  const { series, total, categories, categoryCounts, loading, categoriesLoading, loadMore } = useSeries(
    profileId,
    category,
    debouncedSearch,
    sort,
  );
  const { lockedIds, guardSelect } = useGuardedCategories(categories);

  const playingEpisode =
    session.vodInfo?.itemType === "episode" ? session.vodInfo : null;

  const [episodePanel, setEpisodePanel] = useState<{
    seriesName: string;
    season: number;
    episodes: Episode[];
  } | null>(null);
  const [episodesLoading, setEpisodesLoading] = useState(false);

  useEffect(() => {
    if (!playingEpisode?.seriesId || playingEpisode.season == null) {
      setEpisodePanel(null);
      return;
    }

    let cancelled = false;
    setEpisodesLoading(true);

    void api
      .getSeriesDetail(playingEpisode.seriesId, {
        enrich: false,
        includeSimilar: false,
        syncEpisodes: false,
      })
      .then((detail) => {
        if (cancelled) return;
        const season = playingEpisode.season!;
        const episodes = detail.episodes
          .filter((episode) => episode.season === season)
          .sort((left, right) => left.episode - right.episode);
        setEpisodePanel({
          seriesName: detail.series.name,
          season,
          episodes,
        });
      })
      .catch(() => {
        if (!cancelled) {
          setEpisodePanel(null);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setEpisodesLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [
    playingEpisode?.seriesId,
    playingEpisode?.season,
    playingEpisode?.itemId,
  ]);

  useEffect(() => {
    if (locationState?.category) {
      setCategory(locationState.category);
    }
    if (locationState?.seriesId) {
      setPlayingSeriesId(locationState.seriesId);
    }
  }, [locationState?.category, locationState?.seriesId]);

  const vodState = locationState?.vod;

  useEffect(() => {
    if (!vodState) return;

    const key = `${vodState.itemId}:${vodState.streamUrl}`;

    if (playedVodRef.current === key) return;

    playedVodRef.current = key;

    const nextCategory = locationState?.category ?? "all";
    setCategory(nextCategory);

    if (locationState?.seriesId) {
      setPlayingSeriesId(locationState.seriesId);
    }

    setBrowseSeriesCatalog(false);
    playVod(vodState);

    navigate("/series", { replace: true, state: null });
  }, [
    vodState,
    locationState?.category,
    locationState?.seriesId,
    playVod,
    navigate,
  ]);

  const handleCategorySelect = (categoryId: string) => {
    guardSelect(categoryId, (nextCategory) => {
      setCategory(nextCategory);
      setSearch("");
      setBrowseSeriesCatalog(true);
      setPlayingSeriesId(null);
    });
  };

  const handleOpenSeries = (item: Series) => {
    navigate(`/series/${item.id}`, {
      state: {
        category: category ?? item.category ?? undefined,
        preview: {
          id: item.id,
          name: item.name,
          poster: item.poster ?? null,
          backdrop: item.backdrop ?? null,
        },
      },
    });
  };

  const showItems = category !== null;

  const searchPlaceholder = showItems
    ? `Buscar em ${formatGroupLabel(category!)}`
    : "Selecione uma categoria para buscar";

  const seriesById = new Map(series.map((item) => [item.id, item]));
  const showEpisodePanel =
    !browseSeriesCatalog && episodePanel != null && playingEpisode != null;

  const handlePlaySeasonEpisode = (episode: Episode) => {
    if (!playingEpisode?.seriesId || !profileId || !episodePanel) return;

    setBrowseSeriesCatalog(false);

    const navItems = buildEpisodeNavItems(
      episodePanel.episodes,
      episodePanel.seriesName,
      episodePanel.season,
    );
    const navIndex = navItems.findIndex((item) => item.itemId === episode.id);

    playVod({
      name: `${episodePanel.seriesName} — T${episode.season} E${episode.episode}`,
      streamUrl: episode.streamUrl,
      profileId,
      itemType: "episode",
      itemId: episode.id,
      poster: playingEpisode.poster,
      seriesId: playingEpisode.seriesId,
      season: episode.season,
      episodeNumber: episode.episode,
      navItems,
      navIndex: navIndex >= 0 ? navIndex : undefined,
    });
  };

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
              disabled={!showItems || showEpisodePanel}
              placeholder={searchPlaceholder}
            />
          </div>

          {showEpisodePanel ? (
            <>
              <CategoryContentHeader
                categoryLabel={episodePanel.seriesName}
                subtitle={`Temporada ${episodePanel.season} · ${episodePanel.episodes.length} episódios`}
                icon={
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.75"
                    className="h-4 w-4 text-text-secondary"
                  >
                    <rect x="3" y="4" width="18" height="14" rx="2" />
                    <path d="M7 8h10M7 12h7" strokeLinecap="round" />
                  </svg>
                }
              />
              <VodList
                items={episodePanel.episodes.map((episode) => ({
                  id: episode.id,
                  title: `Episódio ${episode.episode}`,
                  poster: playingEpisode?.poster ?? null,
                  backdrop: null,
                }))}
                total={episodePanel.episodes.length}
                loading={episodesLoading}
                activeItemId={playingEpisode?.itemId ?? null}
                emptyMessage="Nenhum episódio nesta temporada"
                onItemClick={(item) => {
                  const episode = episodePanel.episodes.find(
                    (entry) => entry.id === item.id,
                  );
                  if (episode) {
                    handlePlaySeasonEpisode(episode);
                  }
                }}
                onLoadMore={() => undefined}
              />
            </>
          ) : showItems ? (
            <>
            <CategoryContentHeader
              categoryLabel={formatGroupLabel(category)}
              subtitle={`${activeProfile?.name ?? "Nenhuma lista"} · ${total.toLocaleString()} séries`}
              actions={<CatalogSortPills value={sort} onChange={setSort} />}
              icon={
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.75"
                  className="h-4 w-4 text-text-secondary"
                >
                  <rect x="3" y="4" width="18" height="14" rx="2" />
                  <path d="M7 8h10M7 12h7" strokeLinecap="round" />
                </svg>
              }
            />

            <VodList
              items={series.map((item) => ({
                id: item.id,
                title: item.name?.trim() || item.id,
                poster: item.poster,
                backdrop: item.backdrop,
              }))}
              total={total}
              loading={loading}
              activeItemId={playingSeriesId}
              emptyMessage={
                debouncedSearch.trim()
                  ? `Nenhuma série encontrada para "${debouncedSearch.trim()}"`
                  : category === "all"
                    ? "Nenhuma série encontrada"
                    : `Nenhuma série em "${category}"`
              }
              onItemClick={(item) => {
                const show = seriesById.get(item.id);
                if (show) handleOpenSeries(show);
              }}
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
                <rect x="3" y="4" width="18" height="14" rx="2" />
                <path d="M7 8h10M7 12h7" strokeLinecap="round" />
              </svg>
            </div>
            <p className="text-sm text-text-secondary">Selecione uma categoria</p>
            <p className="max-w-xs text-xs text-text-muted">
              Escolha Drama, Ação, Comédia ou outra categoria para ver as séries
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
