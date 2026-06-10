import type { ReactNode } from "react";
import { useEffect, useState } from "react";

import { cardPosterUrl, heroBackdropUrl } from "../../lib/posterUtils";

interface VodDetailHeroProps {
  backdrop?: string | null;
  poster?: string | null;
  backButton: ReactNode;
  children: ReactNode;
  backdropPending?: boolean;
}

export function VodDetailHero({
  backdrop,
  poster,
  backButton,
  children,
  backdropPending = false,
}: VodDetailHeroProps) {
  const [backdropReady, setBackdropReady] = useState(false);
  const [backdropFailed, setBackdropFailed] = useState(false);

  // Any backdrop (TMDB or provider art) beats the blurred-poster fallback;
  // onError drops back to the blur if the URL turns out to be broken. Shown
  // immediately — waiting for load events kept the hero dark for seconds.
  const backdropUrl = !backdropFailed ? heroBackdropUrl(backdrop) : null;
  const posterFallback = poster ? cardPosterUrl(poster, null) : null;

  useEffect(() => {
    setBackdropReady(false);
    setBackdropFailed(false);
  }, [backdrop]);

  return (
    <section className="relative overflow-hidden">
      <div className="vod-detail-backdrop-frame relative w-full overflow-hidden bg-base-950">
        {/* Poster base layer: keeps the hero from ever rendering black while
            the backdrop downloads, fails to load, or doesn't exist at all. */}
        {posterFallback && (!backdropUrl || !backdropReady) ? (
          <>
            {/* Heavy blur turns the poster into ambient art instead of a
                recognizable zoomed crop while the real backdrop loads. */}
            <img
              src={posterFallback}
              alt=""
              decoding="async"
              referrerPolicy="no-referrer"
              fetchPriority="high"
              className="absolute inset-0 h-full w-full scale-110 object-cover object-center blur-2xl"
            />
            <div className="absolute inset-0 bg-base-950/55" aria-hidden />
          </>
        ) : null}
        {!posterFallback && !backdropUrl ? (
          <div className="hero-gradient absolute inset-0" />
        ) : null}
        {backdropUrl ? (
          // Always visible: the JPEG paints progressively over the blurred
          // poster layer, so the backdrop shows as soon as bytes arrive
          // instead of waiting for a load event that may never settle.
          <img
            src={backdropUrl}
            alt=""
            decoding="async"
            referrerPolicy="no-referrer"
            fetchPriority="high"
            onLoad={() => setBackdropReady(true)}
            onError={() => setBackdropFailed(true)}
            className="vod-detail-backdrop-image absolute inset-0 h-full w-full"
          />
        ) : null}
        {backdropPending && !backdropReady && !posterFallback ? (
          <div
            className="absolute inset-0 animate-pulse bg-base-900/70"
            aria-hidden
          />
        ) : null}
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-base-950 via-base-950/25 to-base-950/5" />
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-r from-base-950/70 via-base-950/25 to-transparent" />
        <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(to_bottom,transparent_55%,rgba(15,15,23,0.85)_100%)]" />
        <div className="absolute left-8 top-6 z-10">{backButton}</div>
      </div>

      <div className="relative -mt-28 px-8 pb-10 md:-mt-32 lg:-mt-36">{children}</div>
    </section>
  );
}
