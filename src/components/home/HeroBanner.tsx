import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

import { FavoriteToggle } from "../favorites/FavoriteToggle";
import { PlayLogoIcon } from "../layout/icons";
import type { HeroSlide } from "../../hooks/useReleaseHero";
import { cardPosterUrl, heroBackdropUrl } from "../../lib/posterUtils";
import { extractYearFromTitle, formatRating } from "../../lib/titleMeta";

const AUTO_ADVANCE_MS = 8000;

interface HeroBannerProps {
  slides?: HeroSlide[];
  loading?: boolean;
  hasProfile?: boolean;
}

function truncateText(text: string, maxLength: number): string {
  const trimmed = text.trim();
  if (trimmed.length <= maxLength) return trimmed;
  return `${trimmed.slice(0, maxLength).trimEnd()}…`;
}

export function HeroBanner({
  slides = [],
  loading = false,
  hasProfile = false,
}: HeroBannerProps) {
  const navigate = useNavigate();
  const [activeIndex, setActiveIndex] = useState(0);
  const [paused, setPaused] = useState(false);

  const slideCount = slides.length;
  const activeSlide = slideCount > 0 ? slides[activeIndex] : null;

  const goTo = useCallback(
    (index: number) => {
      if (slideCount === 0) return;
      setActiveIndex(((index % slideCount) + slideCount) % slideCount);
    },
    [slideCount],
  );

  useEffect(() => {
    setActiveIndex(0);
  }, [slides]);

  useEffect(() => {
    if (slideCount <= 1 || paused) return;

    const timer = window.setInterval(() => {
      setActiveIndex((current) => ((current + 1) % slideCount + slideCount) % slideCount);
    }, AUTO_ADVANCE_MS);
    return () => window.clearInterval(timer);
  }, [slideCount, paused]);

  const openDetail = useCallback(
    (slide: HeroSlide) => {
      // Pass the slide data as preview so the detail page paints poster and
      // backdrop instantly, same as cards clicked in the carousels.
      navigate(`/movies/${slide.id}`, {
        state: {
          preview: {
            id: slide.id,
            name: slide.name,
            poster: slide.poster ?? null,
            backdrop: slide.backdrop ?? null,
            streamUrl: slide.streamUrl,
            profileId: slide.profileId,
            plot: slide.plot ?? null,
            genres: slide.genres ?? null,
            rating: slide.rating ?? null,
          },
        },
      });
    },
    [navigate],
  );

  const handleAssistir = () => {
    if (!activeSlide) return;
    openDetail(activeSlide);
  };

  const handleBrowse = () => {
    navigate("/movies");
  };

  const heroImage = activeSlide
    ? heroBackdropUrl(activeSlide.backdrop) ??
      (activeSlide.poster ? cardPosterUrl(activeSlide.poster, null) : null)
    : null;

  const prefetchIndices = useMemo(() => {
    if (slideCount <= 1) return new Set([activeIndex]);
    const next = (activeIndex + 1) % slideCount;
    return new Set([activeIndex, next]);
  }, [activeIndex, slideCount]);
  const year = activeSlide ? extractYearFromTitle(activeSlide.name) : null;
  const rating = activeSlide ? formatRating(activeSlide.rating) : null;
  const meta = activeSlide
    ? [year, activeSlide.genres, rating].filter(Boolean).join(" • ")
    : null;

  const synopsisText = activeSlide
    ? activeSlide.plot?.trim() ||
      "Sinopse indisponível para este título no momento."
    : null;

  return (
    <section
      className="relative h-[520px] w-full shrink-0 overflow-hidden bg-base-950 md:h-[580px] lg:h-[620px]"
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
      onFocusCapture={() => setPaused(true)}
      onBlurCapture={() => setPaused(false)}
    >
      <div className="hero-backdrop-stack absolute inset-0" aria-hidden>
        {slides.map((slide, index) => {
          if (!prefetchIndices.has(index)) return null;

          const image =
            heroBackdropUrl(slide.backdrop) ??
            (slide.poster ? cardPosterUrl(slide.poster, null) : null);
          if (!image) return null;

          return (
            <div
              key={slide.id}
              className={`absolute inset-x-0 top-0 transition-opacity duration-1000 ease-in-out ${
                index === activeIndex ? "opacity-100" : "opacity-0"
              }`}
            >
              <div className="vod-detail-backdrop-frame relative w-full overflow-hidden bg-base-950">
                <img
                  src={image}
                  alt=""
                  className="vod-detail-backdrop-image absolute inset-0 h-full w-full"
                  decoding="async"
                  fetchPriority={index === 0 ? "high" : "low"}
                />
              </div>
            </div>
          );
        })}

        {!heroImage && !loading && (
          <div className="hero-brand-fallback absolute inset-0" />
        )}

        {loading && (
          <div className="hero-brand-fallback absolute inset-0 animate-pulse" />
        )}

        <div className="hero-mesh-accent absolute inset-0" />
        <div className="hero-overlay-left absolute inset-0" />
        <div className="hero-overlay-bottom absolute inset-0" />
        <div className="hero-overlay-vignette absolute inset-0" />
      </div>

      <div className="relative z-10 flex h-full flex-col justify-end px-8 pb-12 pt-28 md:px-12 md:pb-16 md:pt-36">
        {activeSlide ? (
          <div
            key={activeSlide.id}
            className="hero-slide-content max-w-3xl"
          >
            <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-accent/30 bg-accent/15 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em] text-accent backdrop-blur-sm">
              <span className="h-1.5 w-1.5 rounded-full bg-accent shadow-[0_0_8px_rgba(123,92,255,0.8)]" />
              Lançamento
            </div>

            <h1 className="mb-3 line-clamp-2 min-h-[2.2em] text-4xl font-bold leading-[1.05] tracking-tight md:min-h-[2.15em] md:text-5xl lg:min-h-[2.1em] lg:text-[3.4rem]">
              {activeSlide.name}
            </h1>

            <p
              className={`mb-4 min-h-5 text-sm font-medium md:min-h-6 md:text-base ${
                meta ? "text-text-secondary" : "invisible"
              }`}
              aria-hidden={!meta}
            >
              {meta || "—"}
            </p>

            <p
              className={`mb-3 line-clamp-3 min-h-[4.5rem] max-w-2xl text-sm leading-relaxed md:min-h-[5.25rem] md:text-[15px] md:leading-7 ${
                synopsisText ? "text-text-secondary/95" : "invisible"
              }`}
              aria-hidden={!synopsisText}
            >
              {synopsisText ? truncateText(synopsisText, 280) : "—"}
            </p>

            <p
              className={`mb-6 line-clamp-1 min-h-5 max-w-2xl text-xs md:min-h-6 md:text-sm ${
                activeSlide.cast ? "text-text-muted" : "invisible"
              }`}
              aria-hidden={!activeSlide.cast}
            >
              <span className="font-medium text-text-secondary">Elenco:</span>{" "}
              {activeSlide.cast ? truncateText(activeSlide.cast, 120) : "—"}
            </p>

            <div className="flex flex-wrap items-center gap-3">
              <button
                type="button"
                onClick={handleAssistir}
                className="inline-flex items-center gap-2.5 rounded-full bg-white px-7 py-3 text-base font-semibold text-base-950 shadow-lg shadow-black/20 transition-all duration-250 hover:bg-white/90"
              >
                <PlayLogoIcon className="h-5 w-5 text-base-950" />
                Assistir
              </button>
              <button
                type="button"
                onClick={() => openDetail(activeSlide)}
                className="inline-flex items-center gap-2 rounded-full border border-white/25 bg-white/10 px-6 py-3 text-base font-medium text-text-primary backdrop-blur-sm transition-all duration-250 hover:bg-white/20"
              >
                Saiba mais
              </button>
              <FavoriteToggle
                itemType="movie"
                itemId={activeSlide.id}
                className="border-white/25 bg-white/10 backdrop-blur-sm hover:border-accent/50 hover:bg-white/15"
              />
            </div>
          </div>
        ) : loading ? (
          <div className="max-w-3xl space-y-4">
            <div className="h-6 w-32 animate-pulse rounded-full bg-base-800/80" />
            <div className="h-14 w-4/5 max-w-xl animate-pulse rounded-lg bg-base-800/80" />
            <div className="h-4 w-56 animate-pulse rounded bg-base-800/70" />
            <div className="space-y-2 pt-2">
              <div className="h-3 w-full max-w-2xl animate-pulse rounded bg-base-800/60" />
              <div className="h-3 w-5/6 max-w-xl animate-pulse rounded bg-base-800/60" />
            </div>
            <div className="flex gap-3 pt-2">
              <div className="h-12 w-36 animate-pulse rounded-full bg-base-800/80" />
              <div className="h-12 w-32 animate-pulse rounded-full bg-base-800/60" />
            </div>
          </div>
        ) : (
          <div className="max-w-3xl">
            <p className="mb-3 text-xs font-semibold uppercase tracking-[0.2em] text-text-secondary">
              Bem-vindo
            </p>
            <h1 className="mb-5 text-4xl font-bold leading-tight tracking-tight md:text-5xl lg:text-6xl">
              Play Max
            </h1>
            {hasProfile && (
              <p className="mb-8 max-w-xl text-base leading-relaxed text-text-secondary">
                Explore filmes e séries do seu catálogo.
              </p>
            )}
            <button
              type="button"
              onClick={handleBrowse}
              className="inline-flex items-center gap-2.5 rounded-full bg-white px-7 py-3 text-base font-semibold text-base-950 shadow-lg transition-all duration-250 hover:bg-white/90"
            >
              Explorar catálogo
            </button>
          </div>
        )}
      </div>

      {slideCount > 1 && (
        <div className="absolute bottom-6 right-8 z-20 flex items-center gap-3 md:right-12">
          <div className="flex items-center gap-1.5 rounded-full border border-white/10 bg-base-950/45 px-2 py-1.5 backdrop-blur-md">
            {slides.map((slide, index) => (
              <button
                key={slide.id}
                type="button"
                aria-label={`Ver ${slide.name}`}
                aria-current={index === activeIndex ? "true" : undefined}
                onClick={() => goTo(index)}
                className={`h-2 rounded-full transition-all duration-300 ${
                  index === activeIndex
                    ? "w-7 bg-accent shadow-[0_0_10px_rgba(123,92,255,0.55)]"
                    : "w-2 bg-white/35 hover:bg-white/60"
                }`}
              />
            ))}
          </div>
          <span className="hidden text-xs font-medium tabular-nums text-text-muted sm:inline">
            {activeIndex + 1} / {slideCount}
          </span>
        </div>
      )}

      {slideCount > 1 && (
        <div className="pointer-events-none absolute bottom-0 left-0 right-0 z-10 hidden h-24 bg-gradient-to-t from-base-950/80 to-transparent md:block" />
      )}
    </section>
  );
}
