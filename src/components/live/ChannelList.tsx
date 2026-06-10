import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import type { Channel } from "../../lib/types";
import { ChannelCard } from "./ChannelCard";
import { Skeleton } from "../ui/Skeleton";
import { useChannelsEpg } from "../../hooks/useChannelsEpg";

const CHANNEL_ROW_HEIGHT = 132;
const ROW_GAP = 6;

interface ChannelListProps {
  channels: Channel[];
  total: number;
  loading: boolean;
  profileId: string | null;
  activeChannelId: string | null;
  onPlay: (channel: Channel) => void;
  onLoadMore: () => void;
}

export function ChannelList({
  channels,
  total,
  loading,
  profileId,
  activeChannelId,
  onPlay,
  onLoadMore,
}: ChannelListProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: channels.length,
    getScrollElement: () => parentRef.current,
    getItemKey: (index) => channels[index]?.id ?? index,
    estimateSize: () => CHANNEL_ROW_HEIGHT,
    gap: ROW_GAP,
    overscan: 6,
  });

  const items = virtualizer.getVirtualItems();

  const visibleChannelIds = useMemo(
    () => items.map((item) => channels[item.index]?.id).filter(Boolean) as string[],
    [items, channels],
  );

  const { epgByChannel, loadingIds } = useChannelsEpg(
    profileId,
    visibleChannelIds,
    channels.length > 0,
    4,
  );

  useLayoutEffect(() => {
    const frame = requestAnimationFrame(() => {
      virtualizer.measure();
    });
    return () => cancelAnimationFrame(frame);
  }, [activeChannelId, channels.length, epgByChannel.size, loadingIds.size]);

  useEffect(() => {
    const last = items[items.length - 1];
    if (!last) return;
    if (last.index >= channels.length - 10 && channels.length < total) {
      onLoadMore();
    }
  }, [items, channels.length, total, onLoadMore]);

  if (loading && channels.length === 0) {
    return (
      <div className="flex flex-col gap-2 px-3 py-2">
        {Array.from({ length: 8 }).map((_, i) => (
          <Skeleton key={i} className="h-[124px] w-full rounded-xl" />
        ))}
      </div>
    );
  }

  if (!loading && channels.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 py-12 text-center">
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-base-800 text-text-muted">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-6 w-6">
            <rect x="2" y="5" width="20" height="14" rx="2" />
            <path d="M8 10h8M8 14h5" strokeLinecap="round" />
          </svg>
        </div>
        <p className="text-sm text-text-secondary">Nenhum canal encontrado</p>
        <p className="text-xs text-text-muted">Tente outra categoria ou busca</p>
      </div>
    );
  }

  return (
    <div ref={parentRef} className="panel-scrollbar min-h-0 flex-1 overflow-y-auto px-3 py-2">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {items.map((item) => {
          const channel = channels[item.index];
          return (
            <div
              key={channel.id}
              data-index={item.index}
              ref={virtualizer.measureElement}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                transform: `translateY(${item.start}px)`,
              }}
            >
              <ChannelCard
                channel={channel}
                active={channel.id === activeChannelId}
                epg={epgByChannel.get(channel.id) ?? null}
                epgLoading={loadingIds.has(channel.id)}
                onPlay={onPlay}
              />
            </div>
          );
        })}
      </div>
      {channels.length < total && (
        <p className="py-3 text-center text-[11px] text-text-muted">Carregando mais canais...</p>
      )}
    </div>
  );
}
