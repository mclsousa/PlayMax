import { memo, useEffect, useState } from "react";
import { thumbPosterUrl } from "../../lib/posterUtils";

interface VodListCardProps {
  title: string;
  poster?: string | null;
  backdrop?: string | null;
  active: boolean;
  onClick: () => void;
}

function PlayingIndicator() {
  return (
    <span className="flex h-3.5 items-end gap-0.5" aria-hidden>
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          className="w-0.5 animate-pulse rounded-full bg-accent"
          style={{
            height: `${40 + i * 20}%`,
            animationDelay: `${i * 120}ms`,
          }}
        />
      ))}
    </span>
  );
}

export const VodListCard = memo(function VodListCard({
  title,
  poster,
  backdrop,
  active,
  onClick,
}: VodListCardProps) {
  const [imgError, setImgError] = useState(false);
  const posterSrc = thumbPosterUrl(poster, backdrop);
  const showPoster = Boolean(posterSrc) && !imgError;
  const displayTitle = title.trim() || title;

  useEffect(() => {
    setImgError(false);
  }, [posterSrc]);

  return (
    <button
      type="button"
      onClick={onClick}
      className={`group relative flex w-full items-center gap-3 rounded-xl p-2.5 text-left transition-all duration-200 ${
        active
          ? "bg-gradient-to-r from-accent/20 via-accent/10 to-transparent ring-1 ring-accent/40"
          : "bg-base-900/30 hover:bg-base-800/50"
      }`}
    >
      {active && (
        <span className="absolute left-0 top-1/2 h-8 w-1 -translate-y-1/2 rounded-r-full bg-accent" />
      )}

      <div
        className={`relative h-11 w-11 shrink-0 overflow-hidden rounded-lg bg-base-800 ring-1 ${
          active ? "ring-accent/30" : "ring-base-700/50 group-hover:ring-base-600"
        }`}
      >
        {showPoster ? (
          <img
            src={posterSrc!}
            alt=""
            loading="lazy"
            decoding="async"
            referrerPolicy="no-referrer"
            className="h-full w-full object-cover"
            onError={() => setImgError(true)}
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center text-[10px] font-bold text-text-muted">
            VOD
          </div>
        )}
      </div>

      <div className="min-w-0 flex-1">
        <p
          className={`truncate text-sm font-medium ${
            active
              ? "text-text-primary"
              : "text-text-secondary group-hover:text-text-primary"
          }`}
        >
          {displayTitle}
        </p>
      </div>

      {active && (
        <div className="shrink-0 pr-1">
          <PlayingIndicator />
        </div>
      )}
    </button>
  );
});
