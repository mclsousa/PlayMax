import type { ChannelEpg, EpgProgram } from "../../lib/types";

interface ChannelEpgDisplayProps {
  epg: ChannelEpg | null;
  loading?: boolean;
  compact?: boolean;
  fullList?: boolean;
  showProgress?: boolean;
  className?: string;
}

function programProgress(program: EpgProgram, nowSec = Math.floor(Date.now() / 1000)): number {
  if (program.endTs <= program.startTs) return 0;
  return Math.min(
    100,
    Math.max(0, ((nowSec - program.startTs) / (program.endTs - program.startTs)) * 100),
  );
}

function formatTimeRange(startTs: number, endTs: number): string {
  const start = new Date(startTs * 1000);
  const end = new Date(endTs * 1000);
  const fmt = (d: Date) =>
    d.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  return `${fmt(start)} – ${fmt(end)}`;
}

export function ChannelEpgDisplay({
  epg,
  loading = false,
  compact = false,
  fullList = false,
  showProgress = false,
  className = "",
}: ChannelEpgDisplayProps) {
  if (loading) {
    return (
      <div className={`space-y-1.5 ${className}`}>
        <div className="h-3 w-4/5 animate-pulse rounded bg-base-800/80" />
        <div className="h-1 w-full animate-pulse rounded-full bg-base-800/70" />
        <div className="h-3 w-3/5 animate-pulse rounded bg-base-800/60" />
      </div>
    );
  }

  if (!epg) return null;

  if (!epg.available) {
    return (
      <p className={`text-xs text-text-muted ${className}`}>
        {epg.unavailableReason ?? "EPG indisponível"}
      </p>
    );
  }

  if (fullList) {
    if (epg.programs.length === 0) {
      return (
        <p className={`text-xs text-text-muted ${className}`}>
          Sem programação disponível
        </p>
      );
    }

    return (
      <div className={className}>
        <ul className="max-h-72 space-y-1 overflow-y-auto panel-scrollbar pr-1">
          {epg.programs.map((program) => (
            <li
              key={`${program.startTs}-${program.title}`}
              className={`rounded-lg px-2.5 py-2 ${
                program.isNow ? "bg-accent/10 ring-1 ring-accent/25" : "bg-base-900/50"
              }`}
            >
              <p
                className={`text-sm leading-snug ${
                  program.isNow ? "font-medium text-text-primary" : "text-text-secondary"
                }`}
              >
                {program.title}
              </p>
              <p className="mt-0.5 text-xs text-text-muted">
                {formatTimeRange(program.startTs, program.endTs)}
                {program.isNow ? " · Agora" : null}
              </p>
            </li>
          ))}
        </ul>
      </div>
    );
  }

  const nowProgram = epg.programs.find((p) => p.isNow) ?? epg.programs[0];
  const nextProgram = epg.programs.find(
    (p) => !p.isNow && (!nowProgram || p.startTs >= nowProgram.endTs),
  );

  if (!nowProgram) {
    return (
      <p className={`text-xs text-text-muted ${className}`}>
        Sem programação disponível
      </p>
    );
  }

  if (compact) {
    const progress = programProgress(nowProgram);

    return (
      <div className={`space-y-1 ${className}`}>
        <p className="line-clamp-2 text-[11px] leading-snug text-text-muted">
          <span className="text-text-secondary">Agora:</span> {nowProgram.title}
        </p>
        {showProgress ? (
          <div className="h-1 overflow-hidden rounded-full bg-base-700/80">
            <div
              className="h-full rounded-full bg-accent"
              style={{ width: `${progress}%` }}
            />
          </div>
        ) : null}
        {nextProgram ? (
          <p className="line-clamp-1 text-[11px] leading-snug text-text-muted">
            <span className="text-text-secondary">Depois:</span> {nextProgram.title}
          </p>
        ) : null}
      </div>
    );
  }

  return (
    <div className={`space-y-0.5 ${className}`}>
      <p className="text-sm text-text-secondary">
        <span className="font-medium text-accent">Agora:</span> {nowProgram.title}
        <span className="ml-2 text-xs text-text-muted">
          {formatTimeRange(nowProgram.startTs, nowProgram.endTs)}
        </span>
      </p>
      {nextProgram ? (
        <p className="text-xs text-text-muted">
          <span className="text-text-secondary">Depois:</span> {nextProgram.title}
          <span className="ml-2">
            {formatTimeRange(nextProgram.startTs, nextProgram.endTs)}
          </span>
        </p>
      ) : null}
    </div>
  );
}
