import { useNavigate } from "react-router-dom";
import type { Channel } from "../../lib/types";

interface LiveChannelsRowProps {
  channels: Channel[];
}

interface LiveCardData {
  id: string;
  name: string;
  logo: string | null;
  initials: string;
  color: string;
  now: string;
  next: string;
  progress: number;
}

const PLACEHOLDER_LIVE: LiveCardData[] = [
  { id: "ph-1", name: "Notícias 1", logo: null, initials: "N1", color: "#7b5cff", now: "Jornal da Noite", next: "Mundo", progress: 65 },
  { id: "ph-2", name: "Esporte SP", logo: null, initials: "SP", color: "#2d8f6f", now: "Esporte Total", next: "Resumo", progress: 40 },
  { id: "ph-3", name: "Cinema", logo: null, initials: "CN", color: "#c45c26", now: "Filme: Horizonte", next: "Séries", progress: 80 },
];

const CARD_COLORS = ["#7b5cff", "#2d8f6f", "#c45c26", "#3b6bc4", "#9b3d6b"];

function getInitials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "TV";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[1][0]).toUpperCase();
}

function toCardData(channel: Channel, index: number): LiveCardData {
  const meta = PLACEHOLDER_LIVE[index % PLACEHOLDER_LIVE.length];
  return {
    id: channel.id,
    name: channel.name,
    logo: channel.logo ?? null,
    initials: getInitials(channel.name),
    color: CARD_COLORS[index % CARD_COLORS.length],
    now: meta.now,
    next: meta.next,
    progress: meta.progress,
  };
}

export function LiveChannelsRow({ channels }: LiveChannelsRowProps) {
  const navigate = useNavigate();
  const cards =
    channels.length > 0
      ? channels.slice(0, 3).map(toCardData)
      : PLACEHOLDER_LIVE;

  return (
    <section className="space-y-3">
      <div className="flex items-center gap-3">
        <h2 className="flex items-center gap-2 text-base font-semibold">
          <span className="h-2 w-2 rounded-full bg-red-500" />
          Canais ao vivo
        </h2>
        <span className="text-xs text-text-muted">guia EPG</span>
      </div>

      <div className="grid gap-3 md:grid-cols-3">
        {cards.map((card) => (
          <button
            key={card.id}
            type="button"
            onClick={() => navigate("/live")}
            className="rounded-xl border border-base-800 bg-base-850 p-4 text-left transition-all duration-250 hover:border-base-700 hover:bg-base-800"
          >
            <div className="mb-3 flex items-center gap-3">
              <div
                className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-lg text-xs font-bold text-white"
                style={{ backgroundColor: card.color }}
              >
                {card.logo ? (
                  <img src={card.logo} alt="" className="h-full w-full object-cover" />
                ) : (
                  card.initials
                )}
              </div>
              <span className="font-semibold">{card.name}</span>
            </div>
            <p className="text-sm">
              Agora: <span className="text-text-secondary">{card.now}</span>
            </p>
            <div className="my-2 h-1.5 overflow-hidden rounded-full bg-base-700">
              <div
                className="h-full rounded-full bg-red-500"
                style={{ width: `${card.progress}%` }}
              />
            </div>
            <p className="text-xs text-text-muted">A seguir: {card.next}</p>
          </button>
        ))}
      </div>
    </section>
  );
}
