import { useNavigate } from "react-router-dom";
import type { RecentChannel } from "../../lib/types";

interface RecentChannelsRowProps {
  channels: RecentChannel[];
}

const CARD_COLORS = ["#7b5cff", "#2d8f6f", "#c45c26", "#3b6bc4", "#9b3d6b"];

function getInitials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "TV";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[1][0]).toUpperCase();
}

function formatLastWatched(watchedAt: number): string {
  const nowSec = Math.floor(Date.now() / 1000);
  const diffSec = Math.max(0, nowSec - watchedAt);

  if (diffSec < 60) return "Assistido recentemente";
  if (diffSec < 3600) {
    const min = Math.floor(diffSec / 60);
    return `Última reprodução: há ${min} min`;
  }
  if (diffSec < 86400) {
    const hours = Math.floor(diffSec / 3600);
    return `Última reprodução: há ${hours} h`;
  }
  const days = Math.floor(diffSec / 86400);
  return `Última reprodução: há ${days} dia${days === 1 ? "" : "s"}`;
}

export function RecentChannelsRow({ channels }: RecentChannelsRowProps) {
  const navigate = useNavigate();

  if (channels.length === 0) return null;

  return (
    <section className="space-y-3">
      <div className="flex items-center gap-3">
        <h2 className="flex items-center gap-2 text-base font-semibold">
          <span className="h-2 w-2 rounded-full bg-red-500" />
          Últimos canais
        </h2>
        <span className="text-xs text-text-muted">assistidos recentemente</span>
      </div>

      <div className="grid gap-3 md:grid-cols-3">
        {channels.slice(0, 6).map((channel, index) => {
          const color = CARD_COLORS[index % CARD_COLORS.length];
          const initials = getInitials(channel.name);

          return (
            <button
              key={channel.id}
              type="button"
              onClick={() =>
                navigate("/live", {
                  state: {
                    channel: {
                      id: channel.channelId,
                      profileId: channel.profileId,
                      name: channel.name,
                      logo: channel.logo,
                      streamUrl: channel.streamUrl,
                      sortOrder: 0,
                    },
                  },
                })
              }
              className="rounded-xl border border-base-800 bg-base-850 p-4 text-left transition-all duration-250 hover:border-base-700 hover:bg-base-800"
            >
              <div className="mb-3 flex items-center gap-3">
                <div
                  className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-lg text-xs font-bold text-white"
                  style={{ backgroundColor: color }}
                >
                  {channel.logo ? (
                    <img
                      src={channel.logo}
                      alt=""
                      className="h-full w-full object-cover"
                    />
                  ) : (
                    initials
                  )}
                </div>
                <span className="font-semibold">{channel.name}</span>
              </div>
              <p className="text-sm text-text-secondary">
                {formatLastWatched(channel.watchedAt)}
              </p>
              <p className="mt-2 text-xs text-text-muted">Ao vivo</p>
            </button>
          );
        })}
      </div>
    </section>
  );
}
