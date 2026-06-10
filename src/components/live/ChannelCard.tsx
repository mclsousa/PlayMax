import { memo } from "react";
import { FavoriteToggle } from "../favorites/FavoriteToggle";
import type { Channel, ChannelEpg } from "../../lib/types";
import { ChannelEpgDisplay } from "./ChannelEpgDisplay";

interface ChannelCardProps {
  channel: Channel;
  active: boolean;
  onPlay: (channel: Channel) => void;
  epg?: ChannelEpg | null;
  epgLoading?: boolean;
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

export const ChannelCard = memo(function ChannelCard({
  channel,
  active,
  onPlay,
  epg = null,
  epgLoading = false,
}: ChannelCardProps) {
  return (
    <div
      className={`group relative flex w-full flex-col gap-0.5 rounded-xl p-1.5 transition-all duration-200 ${
        active
          ? "bg-gradient-to-r from-accent/20 via-accent/10 to-transparent ring-1 ring-accent/40"
          : "bg-base-900/30 hover:bg-base-800/50"
      }`}
    >
      <div className="relative flex w-full items-center gap-1">
        <button
          type="button"
          onClick={() => onPlay(channel)}
          className="flex min-w-0 flex-1 items-center gap-3 rounded-lg p-1 text-left"
        >
          <div
            className={`relative h-11 w-11 shrink-0 overflow-hidden rounded-lg bg-base-800 ring-1 ${
              active ? "ring-accent/30" : "ring-base-700/50 group-hover:ring-base-600"
            }`}
          >
            {channel.logo ? (
              <img
                src={channel.logo}
                alt=""
                loading="lazy"
                className="h-full w-full object-cover"
                onError={(e) => {
                  e.currentTarget.style.display = "none";
                }}
              />
            ) : (
              <div className="flex h-full w-full items-center justify-center text-[10px] font-bold text-text-muted">
                TV
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
              {channel.name}
            </p>
            {channel.groupName && (
              <p className="mt-0.5 truncate text-[11px] text-text-muted">
                {channel.groupName}
              </p>
            )}
          </div>
        </button>

        <FavoriteToggle
          itemType="channel"
          itemId={channel.id}
          size="sm"
          stopPropagation
          className={`${active ? "opacity-100" : "opacity-0 group-hover:opacity-100"}`}
        />

        {active && (
          <div className="shrink-0 pr-1">
            <PlayingIndicator />
          </div>
        )}
      </div>

      <div className="px-1 pb-0.5 pl-14">
        <ChannelEpgDisplay
          epg={epg}
          loading={epgLoading}
          compact
          showProgress
        />
      </div>
    </div>
  );
});
