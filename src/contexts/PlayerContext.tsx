import { createContext, useContext, useMemo, useRef } from "react";
import { Outlet } from "react-router-dom";
import { usePlayer as usePlayerState, type MediaKind } from "../hooks/usePlayer";
import type { Channel, VodPlayInfo } from "../lib/types";

type PlayerState = ReturnType<typeof usePlayerState>;

export interface PlayerSessionSnapshot {
  ready: boolean;
  initializing: boolean;
  paused: boolean;
  muted: boolean;
  volume: number;
  playingChannel: Channel | null;
  error: string | null;
  loading: boolean;
  isFullscreen: boolean;
  mediaKind: MediaKind;
  vodInfo: VodPlayInfo | null;
  canNavPrev: boolean;
  canNavNext: boolean;
}

export interface PlayerActionsSnapshot {
  loadStream: PlayerState["loadStream"];
  playVod: PlayerState["playVod"];
  seek: PlayerState["seek"];
  togglePause: PlayerState["togglePause"];
  stop: PlayerState["stop"];
  reload: PlayerState["reload"];
  setPlayerVolume: PlayerState["setPlayerVolume"];
  toggleMute: PlayerState["toggleMute"];
  toggleFullscreen: PlayerState["toggleFullscreen"];
  playNavPrev: PlayerState["playNavPrev"];
  playNavNext: PlayerState["playNavNext"];
}

const PlayerContext = createContext<PlayerState | null>(null);
const PlayerSessionContext = createContext<PlayerSessionSnapshot | null>(null);
const PlayerActionsContext = createContext<PlayerActionsSnapshot | null>(null);

/** Keeps a single player instance alive across /live, /movies and /series. */
export function PlayerProvider() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const player = usePlayerState(videoRef);

  const session = useMemo<PlayerSessionSnapshot>(
    () => ({
      ready: player.ready,
      initializing: player.initializing,
      paused: player.paused,
      muted: player.muted,
      volume: player.volume,
      playingChannel: player.playingChannel,
      error: player.error,
      loading: player.loading,
      isFullscreen: player.isFullscreen,
      mediaKind: player.mediaKind,
      vodInfo: player.vodInfo,
      canNavPrev: player.canNavPrev,
      canNavNext: player.canNavNext,
    }),
    [
      player.ready,
      player.initializing,
      player.paused,
      player.muted,
      player.volume,
      player.playingChannel,
      player.error,
      player.loading,
      player.isFullscreen,
      player.mediaKind,
      player.vodInfo,
      player.canNavPrev,
      player.canNavNext,
    ],
  );

  const actions = useMemo<PlayerActionsSnapshot>(
    () => ({
      loadStream: player.loadStream,
      playVod: player.playVod,
      seek: player.seek,
      togglePause: player.togglePause,
      stop: player.stop,
      reload: player.reload,
      setPlayerVolume: player.setPlayerVolume,
      toggleMute: player.toggleMute,
      toggleFullscreen: player.toggleFullscreen,
      playNavPrev: player.playNavPrev,
      playNavNext: player.playNavNext,
    }),
    [
      player.loadStream,
      player.playVod,
      player.seek,
      player.togglePause,
      player.stop,
      player.reload,
      player.setPlayerVolume,
      player.toggleMute,
      player.toggleFullscreen,
      player.playNavPrev,
      player.playNavNext,
    ],
  );

  return (
    <PlayerActionsContext.Provider value={actions}>
      <PlayerSessionContext.Provider value={session}>
        <PlayerContext.Provider value={player}>
          <Outlet />
        </PlayerContext.Provider>
      </PlayerSessionContext.Provider>
    </PlayerActionsContext.Provider>
  );
}

export function usePlayer(): PlayerState {
  const ctx = useContext(PlayerContext);
  if (!ctx) {
    throw new Error("usePlayer must be used within PlayerProvider");
  }
  return ctx;
}

export function usePlayerSession(): PlayerSessionSnapshot {
  const ctx = useContext(PlayerSessionContext);
  if (!ctx) {
    throw new Error("usePlayerSession must be used within PlayerProvider");
  }
  return ctx;
}

export function usePlayerActions(): PlayerActionsSnapshot {
  const ctx = useContext(PlayerActionsContext);
  if (!ctx) {
    throw new Error("usePlayerActions must be used within PlayerProvider");
  }
  return ctx;
}
