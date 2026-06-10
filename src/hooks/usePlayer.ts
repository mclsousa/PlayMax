import { isTauri } from "@tauri-apps/api/core";

import { getCurrentWindow } from "@tauri-apps/api/window";

import {

  useCallback,

  useEffect,

  useRef,

  useState,

  type RefObject,

} from "react";

import * as api from "../lib/api";

import { BrowserStreamPlayer } from "../lib/browserStreamPlayer";

import type { AutoplayEpisode, VodPlayInfo } from "../lib/types";

import type { Channel } from "../lib/types";



export type MediaKind = "live" | "vod" | null;



const MAX_RETRIES = 3;

const HISTORY_SAVE_INTERVAL_MS = 20_000;



export function usePlayer(videoRef: RefObject<HTMLVideoElement | null>) {

  const [videoElement, setVideoElement] = useState<HTMLVideoElement | null>(null);

  const [ready, setReady] = useState(false);

  const [paused, setPaused] = useState(true);

  const [volume, setVolume] = useState(100);

  const [muted, setMuted] = useState(false);

  const [playingChannel, setPlayingChannel] = useState<Channel | null>(null);

  const [error, setError] = useState<string | null>(null);

  const [loading, setLoading] = useState(false);

  const [isFullscreen, setIsFullscreen] = useState(false);

  const [mediaKind, setMediaKind] = useState<MediaKind>(null);

  const [position, setPosition] = useState(0);

  const [duration, setDuration] = useState(0);

  const [vodInfo, setVodInfo] = useState<VodPlayInfo | null>(null);



  const retryCount = useRef(0);

  const loadingChannelRef = useRef<Channel | null>(null);

  const vodRef = useRef<VodPlayInfo | null>(null);

  const pendingSeekRef = useRef<number | null>(null);

  const positionRef = useRef(0);

  const durationRef = useRef(0);

  const historyIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const positionUiTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const playVodRef = useRef<(info: VodPlayInfo) => void>(() => {});

  const engineRef = useRef(new BrowserStreamPlayer());

  const mountedRef = useRef(true);



  const getVideo = useCallback(() => videoRef.current, [videoRef]);



  const setVideoRef = useCallback((node: HTMLVideoElement | null) => {

    videoRef.current = node;

    setVideoElement(node);

    setReady(node != null);

    if (!node) {

      setPaused(true);

    } else {

      setPaused(node.paused);

    }

  }, [videoRef]);



  const clearHistoryInterval = useCallback(() => {

    if (historyIntervalRef.current) {

      clearInterval(historyIntervalRef.current);

      historyIntervalRef.current = null;

    }

  }, []);



  const persistHistory = useCallback(async (pos: number, dur: number) => {

    const vod = vodRef.current;

    if (!vod || !isTauri()) return;

    try {

      await api.saveHistory({

        profileId: vod.profileId,

        itemType: vod.itemType,

        itemId: vod.itemId,

        name: vod.name,

        poster: vod.poster,

        streamUrl: vod.streamUrl,

        position: pos,

        duration: dur,

      });

    } catch {

      // Best-effort history persistence.

    }

  }, []);



  const startHistoryInterval = useCallback(() => {

    clearHistoryInterval();

    historyIntervalRef.current = setInterval(() => {

      if (vodRef.current) {

        void persistHistory(positionRef.current, durationRef.current);

      }

    }, HISTORY_SAVE_INTERVAL_MS);

  }, [clearHistoryInterval, persistHistory]);



  useEffect(() => {

    mountedRef.current = true;

    if (!videoElement) return;



    const video = videoElement;

    setPaused(video.paused);



    const onTimeUpdate = () => {

      positionRef.current = video.currentTime;

      if (positionUiTimerRef.current) return;

      positionUiTimerRef.current = window.setTimeout(() => {

        positionUiTimerRef.current = null;

        setPosition(positionRef.current);

      }, 1000);

    };

    const onDurationChange = () => {

      const dur = Number.isFinite(video.duration) ? video.duration : 0;

      durationRef.current = dur;

      setDuration(dur);

    };

    const onPlay = () => setPaused(false);

    const onPause = () => setPaused(true);

    const onWaiting = () => setLoading(true);

    const onPlaying = () => {

      setLoading(false);

      setError(null);

      setPaused(false);

    };

    const onError = () => {

      setLoading(false);

      setError("Não foi possível reproduzir o stream.");

    };



    video.addEventListener("timeupdate", onTimeUpdate);

    video.addEventListener("durationchange", onDurationChange);

    video.addEventListener("play", onPlay);

    video.addEventListener("pause", onPause);

    video.addEventListener("waiting", onWaiting);

    video.addEventListener("playing", onPlaying);

    video.addEventListener("error", onError);



    return () => {

      mountedRef.current = false;

      if (positionUiTimerRef.current) {

        window.clearTimeout(positionUiTimerRef.current);

        positionUiTimerRef.current = null;

      }

      video.removeEventListener("timeupdate", onTimeUpdate);

      video.removeEventListener("durationchange", onDurationChange);

      video.removeEventListener("play", onPlay);

      video.removeEventListener("pause", onPause);

      video.removeEventListener("waiting", onWaiting);

      video.removeEventListener("playing", onPlaying);

      video.removeEventListener("error", onError);

    };

  }, [videoElement]);



  useEffect(() => {

    return () => {

      const video = videoRef.current;

      if (video) {

        engineRef.current.detach(video);

      }

      clearHistoryInterval();

      if (retryTimerRef.current) {

        window.clearTimeout(retryTimerRef.current);

        retryTimerRef.current = null;

      }

      document.body.classList.remove("player-fullscreen");

      void getCurrentWindow().setFullscreen(false);

    };

  }, [clearHistoryInterval, videoRef]);



  useEffect(() => {

    if (!isTauri()) return;



    const appWindow = getCurrentWindow();

    let disposed = false;

    let unlisten: (() => void) | undefined;



    void appWindow

      .onResized(async () => {

        if (disposed) return;

        const fs = await appWindow.isFullscreen();

        setIsFullscreen(fs);

        document.body.classList.toggle("player-fullscreen", fs);

      })

      .then((fn) => {

        unlisten = fn;

      });



    return () => {

      disposed = true;

      unlisten?.();

    };

  }, []);



  const loadStream = useCallback(

    async (channel: Channel, isRetry = false, kind: "live" | "vod" = "live") => {

      const video = getVideo();

      if (!video) {

        setError("Área de vídeo indisponível.");

        return;

      }



      setLoading(true);

      setError(null);

      setMediaKind(kind);

      if (kind === "live") {

        vodRef.current = null;

        setVodInfo(null);

        clearHistoryInterval();

      }

      if (!isRetry) {

        setPosition(0);

        setDuration(0);

        positionRef.current = 0;

        durationRef.current = 0;

      }

      setPlayingChannel(channel);

      loadingChannelRef.current = channel;

      if (!isRetry) retryCount.current = 0;



      try {

        await engineRef.current.load(video, channel.streamUrl, kind);

        retryCount.current = 0;

        setLoading(false);

        setPaused(video.paused);



        const seekTo = pendingSeekRef.current;

        if (seekTo != null && seekTo > 0 && kind === "vod") {

          pendingSeekRef.current = null;

          video.currentTime = seekTo;

          positionRef.current = seekTo;

          setPosition(seekTo);

        }



        if (kind === "vod") {

          startHistoryInterval();

        } else if (kind === "live" && isTauri()) {

          void api.saveHistory({

            profileId: channel.profileId,

            itemType: "live",

            itemId: channel.id,

            name: channel.name,

            poster: channel.logo,

            streamUrl: channel.streamUrl,

            position: 0,

            duration: 0,

          });

        }

      } catch (err) {

        setLoading(false);

        const message =

          err instanceof Error ? err.message : "Erro ao reproduzir canal";

        if (retryCount.current < MAX_RETRIES) {

          retryCount.current += 1;

          setError(`Reconectando... (${retryCount.current}/${MAX_RETRIES})`);

          if (retryTimerRef.current) {

            window.clearTimeout(retryTimerRef.current);

          }

          retryTimerRef.current = window.setTimeout(() => {

            retryTimerRef.current = null;

            if (mountedRef.current) {

              void loadStream(channel, true, kind);

            }

          }, 2000);

        } else {

          setError(message);

        }

      }

    },

    [getVideo, clearHistoryInterval, startHistoryInterval],

  );



  const setFullscreen = useCallback(async (enabled: boolean) => {
    if (!isTauri()) return;

    try {
      const appWindow = getCurrentWindow();
      const current = await appWindow.isFullscreen();
      if (current === enabled) {
        setIsFullscreen(enabled);
        document.body.classList.toggle("player-fullscreen", enabled);
        return;
      }

      await appWindow.setFullscreen(enabled);
      setIsFullscreen(enabled);
      document.body.classList.toggle("player-fullscreen", enabled);
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Não foi possível alternar tela cheia.",
      );
    }
  }, []);

  const toggleFullscreen = useCallback(async () => {
    if (!isTauri()) return;

    try {
      const appWindow = getCurrentWindow();
      await setFullscreen(!(await appWindow.isFullscreen()));
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Não foi possível alternar tela cheia.",
      );
    }
  }, [setFullscreen]);



  const playVod = useCallback(

    (info: VodPlayInfo) => {

      const startPos = info.startPosition ?? 0;

      const pseudoChannel: Channel = {

        id: `vod-${info.itemId}`,

        profileId: info.profileId,

        name: info.name,

        streamUrl: info.streamUrl,

        sortOrder: 0,

      };

      vodRef.current = info;

      setVodInfo(info);

      pendingSeekRef.current = startPos > 0 ? startPos : null;

      positionRef.current = startPos;

      durationRef.current = 0;

      void persistHistory(startPos, 0);

      void loadStream(pseudoChannel, false, "vod");

      if (info.startFullscreen) {
        void setFullscreen(true);
      }

    },

    [loadStream, persistHistory, setFullscreen],

  );



  playVodRef.current = playVod;



  const seek = useCallback(

    async (seconds: number) => {

      const video = getVideo();

      if (!video || mediaKind !== "vod") return;

      const dur = Number.isFinite(video.duration) ? video.duration : duration;

      const clamped =

        dur > 0 ? Math.max(0, Math.min(seconds, dur)) : Math.max(0, seconds);

      video.currentTime = clamped;

      positionRef.current = clamped;

      setPosition(clamped);

      void persistHistory(clamped, durationRef.current);

    },

    [getVideo, mediaKind, duration, persistHistory],

  );



  const togglePause = useCallback(async () => {

    const video = getVideo();

    if (!video || !playingChannel) return;

    try {

      if (video.paused) {

        await engineRef.current.play(video);

      } else {

        engineRef.current.pause(video);

      }

    } catch {

      // Fall back to native element state below.

    }

    setPaused(video.paused);

    if (vodRef.current) {

      void persistHistory(positionRef.current, durationRef.current);

    }

  }, [getVideo, playingChannel, persistHistory]);



  const stop = useCallback(async () => {

    const video = getVideo();

    if (!video) return;

    if (vodRef.current) {

      await persistHistory(positionRef.current, durationRef.current);

    }

    clearHistoryInterval();

    vodRef.current = null;

    setVodInfo(null);

    pendingSeekRef.current = null;

    engineRef.current.detach(video);

    setPlayingChannel(null);

    loadingChannelRef.current = null;

    setMediaKind(null);

    setPosition(0);

    setDuration(0);

    positionRef.current = 0;

    durationRef.current = 0;

    setPaused(true);

    setLoading(false);

  }, [getVideo, clearHistoryInterval, persistHistory]);



  const reload = useCallback(async () => {

    if (!playingChannel) return;

    await loadStream(playingChannel, false, mediaKind ?? "live");

  }, [playingChannel, loadStream, mediaKind]);



  const setPlayerVolume = useCallback(

    (value: number) => {

      const video = getVideo();

      if (!video) return;

      const clamped = Math.max(0, Math.min(100, value));

      video.volume = clamped / 100;

      video.muted = clamped === 0;

      setVolume(clamped);

      setMuted(clamped === 0);

    },

    [getVideo],

  );



  const toggleMute = useCallback(() => {

    const video = getVideo();

    if (!video) return;

    if (video.muted || video.volume === 0) {

      const restored = volume > 0 ? volume : 75;

      video.volume = restored / 100;

      video.muted = false;

      setVolume(restored);

      setMuted(false);

    } else {

      video.muted = true;

      setMuted(true);

    }

  }, [getVideo, volume]);



  const buildAutoplayFromNav = useCallback(

    (vod: VodPlayInfo, index: number): AutoplayEpisode[] | undefined => {

      if (!vod.navItems || index < 0) return vod.autoplayQueue;

      return vod.navItems.slice(index + 1).map((item) => ({

        itemId: item.itemId,

        name: item.name,

        streamUrl: item.streamUrl,

      }));

    },

    [],

  );



  const playNavRelative = useCallback(

    (delta: number) => {

      const vod = vodRef.current;

      if (!vod?.navItems || vod.navIndex == null) return;

      const newIndex = vod.navIndex + delta;

      if (newIndex < 0 || newIndex >= vod.navItems.length) return;

      const item = vod.navItems[newIndex];

      playVodRef.current({

        ...vod,

        name: item.name,

        streamUrl: item.streamUrl,

        itemId: item.itemId,

        navIndex: newIndex,

        autoplayQueue: buildAutoplayFromNav(vod, newIndex),

      });

    },

    [buildAutoplayFromNav],

  );



  const canNavPrev =

    vodInfo?.navItems != null &&

    vodInfo.navIndex != null &&

    vodInfo.navIndex > 0;



  const canNavNext =

    vodInfo?.navItems != null &&

    vodInfo.navIndex != null &&

    vodInfo.navIndex < vodInfo.navItems.length - 1;



  return {

    ready,

    initializing: false,

    paused,

    volume,

    muted,

    playingChannel,

    error,

    loading,

    isFullscreen,

    mediaKind,

    position,

    duration,

    loadStream,

    playVod,

    seek,

    togglePause,

    stop,

    reload,

    setPlayerVolume,

    toggleMute,

    toggleFullscreen,

    notifyViewportReady: () => undefined,

    vodInfo,

    canNavPrev,

    canNavNext,

    playNavPrev: () => playNavRelative(-1),

    playNavNext: () => playNavRelative(1),

    videoRef,

    setVideoRef,

    videoElement,

  };

};


