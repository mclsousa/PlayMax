import { useEffect, useRef, useState } from "react";

import { useLocation, useNavigate } from "react-router-dom";

import { CatalogCategorySearch } from "../components/categories/CatalogCategorySearch";
import { CategoryContentHeader } from "../components/categories/CategoryContentHeader";

import { CategoryCards, formatGroupLabel } from "../components/live/CategoryCards";

import { ChannelList } from "../components/live/ChannelList";

import { PlayerOverlay } from "../components/player/PlayerOverlay";

import { useChannels } from "../hooks/useChannels";

import { useDebouncedValue } from "../hooks/useDebouncedValue";
import { useGuardedCategories } from "../hooks/useGuardedCategories";

import { usePlayerActions, usePlayerSession } from "../contexts/PlayerContext";

import * as api from "../lib/api";

import { Skeleton } from "../components/ui/Skeleton";
import { useActiveProfile } from "../hooks/useActiveProfile";

import { useChannelEpg } from "../hooks/useChannelEpg";
import { buildChannelNavigation } from "../lib/playerNavigation";
import type { Channel, VodPlayInfo } from "../lib/types";
import { vodPlayPath } from "../lib/vodRoutes";

interface LiveLocationState {
  vod?: VodPlayInfo;
  channel?: Channel;
  channelId?: string;
  group?: string;
}

export function LivePage() {
  const location = useLocation();
  const navigate = useNavigate();
  const playedChannelRef = useRef<string | null>(null);

  const { activeProfile } = useActiveProfile();

  const [group, setGroup] = useState("all");
  const [showChannels, setShowChannels] = useState(false);
  const [search, setSearch] = useState("");
  const debouncedSearch = useDebouncedValue(search, 300);
  const shouldLoadChannels = showChannels || debouncedSearch.trim().length > 0;

  const { channels, total, groups, categoryCounts, loading, groupsLoading, loadMore } = useChannels(
    activeProfile?.id ?? null,
    group,
    debouncedSearch,
    shouldLoadChannels,
  );

  const session = usePlayerSession();
  const { loadStream } = usePlayerActions();
  const locationState = location.state as LiveLocationState | null;
  const vodState = locationState?.vod;
  const channelState = locationState?.channel;
  const pendingChannelId = locationState?.channelId;
  const pendingGroup = locationState?.group;

  useEffect(() => {
    if (!pendingGroup) return;
    setGroup(pendingGroup);
    setShowChannels(true);
  }, [pendingGroup]);

  useEffect(() => {
    if (!pendingChannelId) return;
    let cancelled = false;
    void api.getChannel(pendingChannelId).then((channel) => {
      if (cancelled || !channel) return;
      const key = `${channel.id}:${channel.streamUrl}`;
      if (playedChannelRef.current === key) return;
      playedChannelRef.current = key;
      void loadStream(channel);
      navigate("/live", { replace: true, state: null });
    });
    return () => {
      cancelled = true;
    };
  }, [pendingChannelId, loadStream, navigate]);

  useEffect(() => {
    if (!vodState) return;
    navigate(vodPlayPath(vodState), { replace: true, state: { vod: vodState } });
  }, [vodState, navigate]);

  useEffect(() => {
    if (!channelState) return;
    const key = `${channelState.id}:${channelState.streamUrl}`;
    if (playedChannelRef.current === key) return;
    playedChannelRef.current = key;
    void loadStream(channelState);
    navigate("/live", { replace: true, state: null });
  }, [channelState, loadStream, navigate]);

  useEffect(() => {
    if (!debouncedSearch.trim()) return;
    setShowChannels(true);
  }, [debouncedSearch]);

  const handlePlay = (channel: Channel) => {
    void loadStream(channel);
  };

  const channelNav = buildChannelNavigation(
    channels,
    session.playingChannel?.id ?? null,
  );

  const { epg, loading: epgLoading } = useChannelEpg(
    activeProfile?.id ?? null,
    session.playingChannel?.id ?? null,
    Boolean(session.playingChannel),
    20,
  );

  const { lockedIds, guardSelect } = useGuardedCategories(groups);

  const handleCategorySelect = (categoryId: string) => {
    guardSelect(categoryId, (nextCategory) => {
      setGroup(nextCategory);
      setShowChannels(true);
      setSearch("");
    });
  };

  const searchPlaceholder = showChannels
    ? `Buscar em ${formatGroupLabel(group)}`
    : "Selecione uma categoria para buscar";

  return (
    <div className="player-route live-page flex min-h-0 flex-1 flex-col overflow-hidden">
      <CategoryCards
        groups={groups}
        counts={categoryCounts}
        onSelect={handleCategorySelect}
        orientation="horizontal"
        selectedId={showChannels ? group : null}
        loading={groupsLoading}
        lockedIds={lockedIds}
      />

      <div className="flex min-h-0 flex-1 overflow-hidden">
        <div className="channel-panel flex min-h-0 w-full max-w-md shrink-0 flex-col overflow-hidden border-r border-base-800/80 bg-base-950">
          <div className="shrink-0 border-b border-base-800/80 px-4 py-2.5">
            <CatalogCategorySearch
              value={search}
              onChange={setSearch}
              disabled={!showChannels}
              placeholder={searchPlaceholder}
            />
          </div>

          {showChannels ? (
            <>
              <CategoryContentHeader
                categoryLabel={formatGroupLabel(group)}
                subtitle={`${activeProfile?.name ?? "Nenhuma lista"} · ${total.toLocaleString()} canais`}
                icon={
                  <span className="relative flex h-full w-full items-center justify-center">
                    <span className="absolute inline-flex h-2.5 w-2.5 animate-ping rounded-full bg-red-500/60" />
                    <span className="relative h-2 w-2 rounded-full bg-red-500" />
                  </span>
                }
              />

              <ChannelList
                channels={channels}
                total={total}
                loading={loading}
                profileId={activeProfile?.id ?? null}
                activeChannelId={session.playingChannel?.id ?? null}
                onPlay={handlePlay}
                onLoadMore={() => void loadMore()}
              />
            </>
          ) : groupsLoading ? (
            <div className="flex min-h-0 flex-1 flex-col gap-3 px-4 py-4">
              <Skeleton className="h-14 w-full rounded-xl" />
              {Array.from({ length: 6 }).map((_, index) => (
                <Skeleton key={index} className="h-12 w-full rounded-lg" />
              ))}
            </div>
          ) : (
            <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-8 py-16 text-center">
              <div className="relative flex h-14 w-14 items-center justify-center rounded-2xl bg-base-800/80 ring-1 ring-base-700/50">
                <span className="absolute inline-flex h-3 w-3 animate-ping rounded-full bg-red-500/60" />
                <span className="relative h-2.5 w-2.5 rounded-full bg-red-500" />
              </div>
              <p className="text-sm text-text-secondary">Selecione uma categoria</p>
              <p className="max-w-xs text-xs text-text-muted">
                Escolha uma categoria acima para ver os canais ao vivo
              </p>
            </div>
          )}
        </div>

        <div className="player-video-column flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
          <PlayerOverlay
            canNavPrev={channelNav.canPrev}
            canNavNext={channelNav.canNext}
            onNavPrev={() => channelNav.goPrev(handlePlay)}
            onNavNext={() => channelNav.goNext(handlePlay)}
            epg={epg}
            epgLoading={epgLoading}
          />
        </div>
      </div>
    </div>
  );
}
