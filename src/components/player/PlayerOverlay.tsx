import { memo, useEffect, useRef, useState, useCallback } from "react";

import { usePlayer } from "../../contexts/PlayerContext";
import { useVideoTracks } from "../../hooks/useVideoTracks";

import { PlayerIconButton } from "./PlayerIconButton";
import {
  AudioTrackIcon,
  FullscreenExitIcon,
  FullscreenIcon,
  PauseIcon,
  PlayIcon,
  ReloadIcon,
  SkipNextIcon,
  SkipPrevIcon,
  StopIcon,
  SubtitlesIcon,
  VolumeIcon,
  VolumeMutedIcon,
} from "./PlayerIcons";
import { TrackPopover } from "./TrackPopover";
import { EpgPopover } from "./EpgPopover";
import { ChannelEpgDisplay } from "../live/ChannelEpgDisplay";
import type { ChannelEpg } from "../../lib/types";

function formatTime(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const minutes = Math.floor(total / 60);
  const secs = total % 60;
  return `${minutes}:${secs.toString().padStart(2, "0")}`;
}

interface PlayerOverlayProps {
  canNavPrev?: boolean;
  canNavNext?: boolean;
  onNavPrev?: () => void;
  onNavNext?: () => void;
  epg?: ChannelEpg | null;
  epgLoading?: boolean;
}



function ControlDivider() {

  return <div className="mx-1 h-6 w-px shrink-0 bg-white/10" aria-hidden />;

}



export const PlayerOverlay = memo(function PlayerOverlay({
  canNavPrev: canNavPrevOverride,
  canNavNext: canNavNextOverride,
  onNavPrev: onNavPrevOverride,
  onNavNext: onNavNextOverride,
  epg = null,
  epgLoading = false,
}: PlayerOverlayProps) {
  const player = usePlayer();
  const {
    ready,
    initializing,
    paused,
    muted,
    volume,
    loading,
    error,
    mediaKind,
    position,
    duration,
    isFullscreen,
    canNavPrev: playerCanNavPrev,
    canNavNext: playerCanNavNext,
    playingChannel,
    setVideoRef,
    videoElement,
    togglePause,
    stop,
    reload,
    toggleMute,
    setPlayerVolume,
    seek,
    toggleFullscreen,
    playNavPrev,
    playNavNext,
  } = player;

  const channelName = playingChannel?.name ?? null;
  const canNavPrev = canNavPrevOverride ?? playerCanNavPrev;
  const canNavNext = canNavNextOverride ?? playerCanNavNext;
  const onNavPrev = onNavPrevOverride ?? (() => playNavPrev());
  const onNavNext = onNavNextOverride ?? (() => playNavNext());
  const onTogglePause = () => void togglePause();
  const onStop = () => void stop();
  const onReload = () => void reload();
  const onToggleMute = () => void toggleMute();
  const onVolumeChange = (value: number) => void setPlayerVolume(value);
  const onSeek = (seconds: number) => void seek(seconds);
  const onToggleFullscreen = () => void toggleFullscreen();
  const mediaKey = playingChannel?.streamUrl ?? null;
  const tracks = useVideoTracks(videoElement, ready, mediaKind, mediaKey);
  const showTrackControls = mediaKind === "vod";

  const [showControls, setShowControls] = useState(true);
  const [epgOpen, setEpgOpen] = useState(false);

  const hideTimer = useRef<number | null>(null);

  const isPlaying = Boolean(channelName);
  const showEpgButton = mediaKind === "live" && isPlaying;

  const showSeekBar = mediaKind === "vod" && duration > 0;

  const transportDisabled = !ready || !channelName || loading;

  const revealControls = useCallback((persist = false) => {

    setShowControls(true);

    if (hideTimer.current) window.clearTimeout(hideTimer.current);

    if (!persist && !epgOpen) {
      hideTimer.current = window.setTimeout(() => setShowControls(false), 3000);
    }

  }, [epgOpen]);

  const handleEpgToggle = useCallback((open: boolean) => {
    setEpgOpen(open);
    revealControls(open);
  }, [revealControls]);



  useEffect(() => {

    revealControls();

    return () => {

      if (hideTimer.current) window.clearTimeout(hideTimer.current);

    };

  }, [channelName]);

  useEffect(() => {
    if (isFullscreen) {
      revealControls();
    } else {
      setShowControls(true);
    }
  }, [isFullscreen, revealControls]);

  useEffect(() => {

    const onKey = (e: KeyboardEvent) => {

      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {

        return;

      }

      if (e.code === "Space") {

        e.preventDefault();

        onTogglePause();

      }

      if (e.code === "KeyF") {

        e.preventDefault();

        onToggleFullscreen();

      }

      if (e.code === "Escape" && isFullscreen) {

        e.preventDefault();

        onToggleFullscreen();

      }

      if (e.code === "KeyM") onToggleMute();

    };

    window.addEventListener("keydown", onKey);

    return () => window.removeEventListener("keydown", onKey);

  }, [onTogglePause, onToggleFullscreen, onToggleMute, isFullscreen]);



  return (

    <div

      className="player-shell relative flex min-h-0 flex-1 flex-col overflow-hidden"

      onMouseMove={() => revealControls()}

      onMouseEnter={() => revealControls()}

    >

      <div
        data-mpv-viewport
        className="video-surface relative min-h-0 flex-1 bg-black"
      >
        <video
          ref={setVideoRef}
          className="absolute inset-0 z-0 h-full w-full bg-black object-contain"
          playsInline
        />

        {!isPlaying && (
          <div className="absolute inset-0 z-[1] flex items-center justify-center rounded-2xl border border-base-800 bg-base-950">
            <p className="text-sm text-text-secondary">
              {initializing
                ? "Inicializando player..."
                : "Selecione um canal para assistir"}
            </p>
          </div>
        )}
      </div>



      <div
        className={`pointer-events-none absolute inset-x-0 top-0 z-20 bg-gradient-to-b from-base-950/95 via-base-950/75 to-transparent px-4 pb-8 pt-4 transition-opacity duration-250 ${
          showControls || epgOpen ? "opacity-100" : "opacity-0"
        }`}
      >

        <h2 className="text-xl font-semibold drop-shadow-md">

          {loading ? `Carregando: ${channelName ?? "..."}` : (channelName ?? "Nenhum canal")}

        </h2>

        {mediaKind === "live" && isPlaying ? (
          <ChannelEpgDisplay
            epg={epg}
            loading={epgLoading}
            className="mt-1.5 max-w-xl drop-shadow-md"
          />
        ) : null}

      </div>



      <div

        className={`pointer-events-auto absolute inset-x-0 bottom-0 z-20 p-4 transition-opacity duration-250 ${

          showControls || epgOpen ? "opacity-100" : "opacity-0"

        }`}

      >

        {error && (

          <div className="mb-3 rounded-xl bg-red-950/80 px-3 py-2 text-sm text-red-200 backdrop-blur-md">

            {error}

          </div>

        )}



        {showSeekBar && (

          <div className="mb-2 rounded-xl bg-base-950/75 px-3 py-2.5 backdrop-blur-xl">

            <div className="mb-1.5 flex items-center justify-between text-xs tabular-nums text-text-secondary">

              <span>{formatTime(position)}</span>

              <span>{formatTime(duration)}</span>

            </div>

            <input

              type="range"

              min={0}

              max={duration}

              step={0.1}

              value={Math.min(position, duration)}

              onChange={(e) => onSeek(Number(e.target.value))}

              className="player-range w-full"

              disabled={!ready}

              aria-label="Posição da reprodução"

            />

          </div>

        )}



        <div className="flex items-center gap-1 rounded-xl bg-base-950/75 px-2 py-1.5 backdrop-blur-xl">

          <div className="flex items-center gap-0.5">

            <PlayerIconButton

              label={paused ? "Reproduzir" : "Pausar"}

              onClick={onTogglePause}

              disabled={transportDisabled}

              active={!paused && isPlaying}

            >

              {paused ? <PlayIcon className="size-5" /> : <PauseIcon className="size-5" />}

            </PlayerIconButton>



            <PlayerIconButton label="Parar" onClick={onStop} disabled={transportDisabled}>

              <StopIcon className="size-[18px]" />

            </PlayerIconButton>



            <PlayerIconButton

              label={loading ? "Carregando..." : "Recarregar"}

              onClick={onReload}

              disabled={transportDisabled || loading}

            >

              <ReloadIcon className={`size-5 ${loading ? "animate-spin" : ""}`} />

            </PlayerIconButton>



            {onNavPrev && (
              <PlayerIconButton
                label="Anterior"
                onClick={onNavPrev}
                disabled={transportDisabled || !canNavPrev}
              >
                <SkipPrevIcon className="size-5" />
              </PlayerIconButton>
            )}

            {onNavNext && (
              <PlayerIconButton
                label="Próximo"
                onClick={onNavNext}
                disabled={transportDisabled || !canNavNext}
              >
                <SkipNextIcon className="size-5" />
              </PlayerIconButton>
            )}

          </div>



          {showTrackControls && (
            <>
              <ControlDivider />
              <TrackPopover
                label="Legendas"
                icon={<SubtitlesIcon className="size-5" />}
                tracks={tracks.subTracks}
                activeId={tracks.activeSid}
                disabled={transportDisabled}
                showOffOption
                onSelect={(id) => tracks.selectSubtitle(id)}
              />
            </>
          )}

          {showTrackControls && (
            <>
              <ControlDivider />
              <TrackPopover
                label="Áudio"
                icon={<AudioTrackIcon className="size-5" />}
                tracks={tracks.audioTracks}
                activeId={tracks.activeAid}
                disabled={transportDisabled || tracks.audioTracks.length === 0}
                emptyLabel="Nenhuma faixa de áudio disponível"
                onSelect={(id) => {
                  if (typeof id === "number") tracks.selectAudio(id);
                }}
              />
            </>
          )}

          <ControlDivider />

          <div className="flex items-center gap-1">

            <PlayerIconButton

              label={muted ? "Ativar som" : "Silenciar"}

              onClick={onToggleMute}

              disabled={!ready}

              active={muted}

            >

              {muted ? <VolumeMutedIcon className="size-5" /> : <VolumeIcon className="size-5" />}

            </PlayerIconButton>



            <input

              type="range"

              min={0}

              max={100}

              value={muted ? 0 : volume}

              onChange={(e) => onVolumeChange(Number(e.target.value))}

              className="player-range w-24 sm:w-28"

              disabled={!ready}

              aria-label="Volume"

              title={`Volume: ${muted ? 0 : volume}%`}

            />

          </div>

          <div className="flex-1" />

          {showEpgButton ? (
            <EpgPopover
              channelKey={playingChannel?.id ?? null}
              epg={epg}
              loading={epgLoading}
              disabled={!ready}
              onToggle={handleEpgToggle}
            />
          ) : null}

          <PlayerIconButton

            label={isFullscreen ? "Sair da tela cheia (F)" : "Tela cheia (F)"}

            onClick={onToggleFullscreen}

            disabled={!ready}

            active={isFullscreen}

          >

            {isFullscreen ? (

              <FullscreenExitIcon className="size-5" />

            ) : (

              <FullscreenIcon className="size-5" />

            )}

          </PlayerIconButton>

        </div>

      </div>

    </div>

  );

});

