import { useEffect, useMemo, useState } from "react";

import { useLocation, useNavigate, useParams } from "react-router-dom";

import { FavoriteToggle } from "../components/favorites/FavoriteToggle";

import { PlayLogoIcon } from "../components/layout/icons";

import { Button } from "../components/ui/Button";

import { Skeleton } from "../components/ui/Skeleton";

import {
  ContentCarousel,
  type CarouselItem,
} from "../components/home/ContentCarousel";

import { CastGrid } from "../components/vod/CastGrid";

import { DetailBackButton } from "../components/vod/DetailBackButton";

import { VodDetailHero } from "../components/vod/VodDetailHero";

import * as api from "../lib/api";

import { bestPosterUrl, cardPosterUrl } from "../lib/posterUtils";

import { buildEpisodeNavItems } from "../lib/playerNavigation";
import { extractYearFromTitle, formatRating } from "../lib/titleMeta";
import type { AutoplayEpisode, Episode, Series, SeriesDetail } from "../lib/types";



interface SeriesDetailPreview {
  id: string;
  name: string;
  poster?: string | null;
  backdrop?: string | null;
}

interface SeriesDetailLocationState {
  category?: string;
  preview?: SeriesDetailPreview;
}

function detailFromPreview(preview: SeriesDetailPreview): SeriesDetail {
  return {
    series: {
      id: preview.id,
      profileId: "",
      name: preview.name,
      poster: preview.poster ?? null,
      backdrop: preview.backdrop ?? null,
      plot: null,
      genres: null,
      rating: null,
      category: null,
      addedAt: 0,
      sortOrder: 0,
    },
    episodes: [],
    cast: null,
    similar: null,
  };
}



function sortEpisodes(episodes: Episode[]): Episode[] {

  return [...episodes].sort((a, b) => {

    if (a.season !== b.season) return a.season - b.season;

    return a.episode - b.episode;

  });

}



function buildAutoplayQueue(

  episodes: Episode[],

  current: Episode,

  seriesName: string,

): AutoplayEpisode[] {

  const sorted = sortEpisodes(episodes);

  const index = sorted.findIndex((ep) => ep.id === current.id);

  if (index < 0) return [];

  return sorted.slice(index + 1).map((ep) => ({

    itemId: ep.id,

    name: `${seriesName} — T${ep.season} E${ep.episode}`,

    streamUrl: ep.streamUrl,

  }));

}



export function SeriesDetailPage() {

  const { id } = useParams<{ id: string }>();

  const navigate = useNavigate();

  const location = useLocation();

  const locationState = location.state as SeriesDetailLocationState | null;
  const preview =
    locationState?.preview?.id === id ? locationState?.preview : undefined;

  const [detail, setDetail] = useState<SeriesDetail | null>(() =>
    preview ? detailFromPreview(preview) : null,
  );

  const [loading, setLoading] = useState(!preview);

  const [syncing, setSyncing] = useState(false);

  const [enriching, setEnriching] = useState(Boolean(preview));

  const [error, setError] = useState<string | null>(null);

  const [selectedSeason, setSelectedSeason] = useState<number | null>(null);



  useEffect(() => {

    if (!id) return;

    let cancelled = false;

    // The route only swaps the :id param (e.g. "Semelhantes" cards), so the
    // component never remounts — drop the previous series' state or it stays
    // visible (episodes, seasons, art) while the new one loads.
    setDetail(preview ? detailFromPreview(preview) : null);
    setLoading(!preview);
    setSelectedSeason(null);

    async function load() {
      setError(null);
      if (!preview) {
        setLoading(true);
      }
      setSyncing(true);
      setEnriching(true);

      try {
        const base = await api.getSeriesDetail(id!, {
          enrich: false,
          includeSimilar: false,
          syncEpisodes: false,
        });

        if (cancelled) return;

        setDetail(base);
        setLoading(false);

        const seasons = Array.from(
          new Set(base.episodes.map((ep) => ep.season)),
        ).sort((a, b) => a - b);
        if (seasons.length > 0) {
          setSelectedSeason(seasons[0] ?? null);
        }

        let nextDetail = base;
        if (base.episodes.length === 0) {
          const synced = await api.getSeriesDetail(id!, {
            enrich: false,
            includeSimilar: false,
            syncEpisodes: true,
          });
          if (!cancelled) {
            nextDetail = {
              ...base,
              episodes: synced.episodes,
              cast: base.cast ?? synced.cast,
            };
            setDetail(nextDetail);
            const syncedSeasons = Array.from(
              new Set(synced.episodes.map((ep) => ep.season)),
            ).sort((a, b) => a - b);
            setSelectedSeason((current) => current ?? syncedSeasons[0] ?? null);
          }
        }

        const enriched = await api.getSeriesDetail(id!, {
          enrich: true,
          includeSimilar: false,
          syncEpisodes: false,
        });

        if (cancelled) return;

        const merged: SeriesDetail = {
          ...nextDetail,
          series: {
            ...nextDetail.series,
            backdrop: enriched.series.backdrop ?? nextDetail.series.backdrop,
            poster: enriched.series.poster ?? nextDetail.series.poster,
            plot: enriched.series.plot ?? nextDetail.series.plot,
            genres: enriched.series.genres ?? nextDetail.series.genres,
            rating: enriched.series.rating ?? nextDetail.series.rating,
          },
          cast: enriched.cast ?? nextDetail.cast,
        };

        setDetail(merged);

        const withSimilar = await api.getSeriesDetail(id!, {
          enrich: false,
          includeSimilar: true,
          syncEpisodes: false,
        });
        if (!cancelled) {
          setDetail((current) =>
            current
              ? { ...current, similar: withSimilar.similar ?? current.similar }
              : withSimilar,
          );
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

          setSyncing(false);
          setEnriching(false);

        }

      }

    }



    void load();

    return () => {

      cancelled = true;

    };

  }, [id, preview]);



  const episodesBySeason = useMemo(() => {

    if (!detail) return new Map<number, Episode[]>();

    const map = new Map<number, Episode[]>();

    for (const ep of sortEpisodes(detail.episodes)) {

      const list = map.get(ep.season) ?? [];

      list.push(ep);

      map.set(ep.season, list);

    }

    return map;

  }, [detail]);



  const seasons = useMemo(

    () => Array.from(episodesBySeason.keys()).sort((a, b) => a - b),

    [episodesBySeason],

  );

  const similarItems = useMemo<CarouselItem[]>(
    () =>
      (detail?.similar ?? []).map((item) => ({
        id: item.id,
        title: item.name,
        posterUrl: bestPosterUrl(item.poster, item.backdrop),
        backdropUrl: item.backdrop ?? null,
      })),
    [detail?.similar],
  );



  const handlePlayEpisode = (episode: Episode, series: Series) => {
    const label = `${series.name} — T${episode.season} E${episode.episode}`;
    const category = locationState?.category ?? series.category ?? undefined;
    const episodes = detail?.episodes ?? [];
    const seasonEpisodes = episodes.filter((ep) => ep.season === episode.season);
    const navItems = buildEpisodeNavItems(
      episodes,
      series.name,
      episode.season,
    );
    const navIndex = navItems.findIndex((item) => item.itemId === episode.id);

    navigate("/series", {
      state: {
        vod: {
          name: label,
          streamUrl: episode.streamUrl,
          profileId: series.profileId,
          itemType: "episode",
          itemId: episode.id,
          poster: series.poster,
          seriesId: series.id,
          season: episode.season,
          episodeNumber: episode.episode,
          navItems,
          navIndex: navIndex >= 0 ? navIndex : undefined,
          autoplayQueue: buildAutoplayQueue(seasonEpisodes, episode, series.name),
        },
        category,
        seriesId: series.id,
      },
    });
  };



  if (loading) {

    return (

      <div className="flex h-full flex-col overflow-hidden app-bg">

        <div className="flex-1 overflow-y-auto bg-base-950 px-8 pb-8">

          <Skeleton className="mb-6 h-64 w-full rounded-2xl" />

          <Skeleton className="mb-3 h-8 w-2/3 max-w-md" />

          <Skeleton className="mb-6 h-4 w-48" />

          {Array.from({ length: 4 }).map((_, i) => (

            <Skeleton key={i} className="mb-2 h-14 w-full max-w-xl rounded-xl" />

          ))}

        </div>

      </div>

    );

  }



  if (error || !detail) {

    return (

      <div className="flex h-full flex-col overflow-hidden app-bg">

        <div className="flex flex-1 flex-col items-center justify-center gap-4 bg-base-950 text-text-secondary">

          <p>{error ?? "Série não encontrada"}</p>

          <Button variant="secondary" onClick={() => navigate("/series")}>

            Voltar às séries

          </Button>

        </div>

      </div>

    );

  }



  const { series, cast } = detail;

  const category = locationState?.category ?? series.category ?? null;
  const year = extractYearFromTitle(series.name);
  const rating = formatRating(series.rating);
  const meta = [category, year, series.genres, rating].filter(Boolean).join(" • ");

  const heroImage = series.backdrop;
  const hasCast = Boolean(cast?.length);
  const hasSynopsisSection = Boolean(series.plot || hasCast);

  const activeSeason = selectedSeason ?? seasons[0] ?? null;

  const seasonEpisodes =

    activeSeason != null ? episodesBySeason.get(activeSeason) ?? [] : [];

  const handleBack = () => {
    navigate("/series", {
      state: category ? { category } : undefined,
    });
  };

  const handleSimilarClick = (item: CarouselItem) => {
    navigate(`/series/${item.id}`, {
      state: {
        category: category ?? undefined,
        preview: {
          id: item.id,
          name: item.title,
          poster: item.posterUrl ?? null,
          backdrop: item.backdropUrl ?? null,
        },
      },
    });
  };



  return (

    <div className="flex h-full flex-col overflow-hidden app-bg">

      <div className="scrollbar-thin flex-1 overflow-y-auto bg-base-950">

        <VodDetailHero
          backdrop={heroImage}
          poster={series.poster}
          backdropPending={enriching}
          backButton={<DetailBackButton onClick={handleBack} />}
        >
          <div className="flex flex-col gap-6 md:flex-row md:items-end">
            {series.poster && (
              <div className="shrink-0">
                <img
                  src={cardPosterUrl(series.poster, null) ?? series.poster}
                  alt={series.name}
                  decoding="async"
                  referrerPolicy="no-referrer"
                  className="h-56 w-40 rounded-xl object-cover shadow-2xl ring-1 ring-base-700/60 md:h-72 md:w-48"
                />
              </div>
            )}
            <div className="flex min-w-0 flex-1 items-start justify-between gap-4">
              <div className="min-w-0 flex-1">
                <h1 className="mb-2 text-4xl font-bold tracking-tight">{series.name}</h1>
                {meta && (
                  <p className="text-sm text-text-secondary">{meta}</p>
                )}
              </div>
              <FavoriteToggle itemType="series" itemId={series.id} />
            </div>
          </div>
        </VodDetailHero>



        {(hasSynopsisSection) ? (
          <div className="space-y-8 px-8 pb-6">
            {series.plot && (
              <div>
                <h2 className="mb-2 text-sm font-semibold uppercase tracking-wider text-text-muted">
                  Sinopse
                </h2>
                <p className="max-w-3xl text-sm leading-relaxed text-text-secondary">
                  {series.plot}
                </p>
              </div>
            )}
            {hasCast && cast && (
              <div>
                <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-text-muted">
                  Elenco
                </h2>
                <CastGrid cast={cast} />
              </div>
            )}
          </div>
        ) : (
          <div className="px-8 pb-6">
            <p className="max-w-3xl text-sm text-text-muted">
              Sinopse indisponível no momento para este título.
            </p>
          </div>
        )}



        <div className="px-8 pb-10">

          {syncing && seasons.length === 0 && (

            <p className="mb-4 text-sm text-text-muted">Buscando episódios...</p>

          )}

          {enriching && seasons.length > 0 && !series.plot && (
            <p className="mb-4 text-sm text-text-muted">Carregando detalhes...</p>
          )}



          {seasons.length > 0 && (

            <div className="mb-6 flex flex-wrap gap-2">

              {seasons.map((season) => (

                <button

                  key={season}

                  type="button"

                  onClick={() => setSelectedSeason(season)}

                  className={`rounded-full px-4 py-1.5 text-sm font-medium transition-colors ${

                    activeSeason === season

                      ? "bg-accent text-white"

                      : "bg-base-800/80 text-text-secondary hover:bg-base-800 hover:text-text-primary"

                  }`}

                >

                  Temporada {season}

                </button>

              ))}

            </div>

          )}



          {activeSeason != null && seasonEpisodes.length > 0 && (

            <ul className="space-y-2">

              {seasonEpisodes.map((episode) => (

                <li key={episode.id}>

                  <button

                    type="button"

                    onClick={() => handlePlayEpisode(episode, series)}

                    className="flex w-full max-w-2xl items-center gap-4 rounded-xl border border-base-700/60 bg-base-850/50 px-4 py-3 text-left transition-all duration-250 hover:border-accent/40 hover:bg-base-800"

                  >

                    <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-accent/20 text-accent">

                      <PlayLogoIcon className="h-4 w-4" />

                    </span>

                    <div className="min-w-0 flex-1">

                      <p className="truncate text-sm font-medium">

                        {episode.episode}. {episode.title}

                      </p>

                      {episode.plot && (

                        <p className="truncate text-xs text-text-muted">

                          {episode.plot}

                        </p>

                      )}

                    </div>

                  </button>

                </li>

              ))}

            </ul>

          )}



          {seasons.length === 0 && !syncing && (

            <p className="text-sm text-text-secondary">Nenhum episódio disponível.</p>

          )}

        </div>

        {similarItems.length > 0 && (
          <div className="px-8 pb-10">
            <ContentCarousel
              title="Títulos semelhantes"
              items={similarItems}
              variant="poster"
              onItemClick={handleSimilarClick}
            />
          </div>
        )}

      </div>

    </div>

  );

}


