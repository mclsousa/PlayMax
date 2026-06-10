import { useCallback, useEffect, useState } from "react";
import type { MpvTrack } from "../lib/mpvTracks";
import {
  applyVideoAudioTrack,
  applyVideoSubtitleTrack,
  readVideoAudioTracks,
  readVideoTextTracks,
} from "../lib/videoTracks";
import type { MediaKind } from "./usePlayer";

const TRACK_PROBE_INTERVAL_MS = 500;

export function useVideoTracks(
  video: HTMLVideoElement | null,
  ready: boolean,
  mediaKind: MediaKind,
  mediaKey: string | null,
) {
  const [audioTracks, setAudioTracks] = useState<MpvTrack[]>([]);
  const [subTracks, setSubTracks] = useState<MpvTrack[]>([]);
  const [activeAid, setActiveAid] = useState<number | null>(null);
  const [activeSid, setActiveSid] = useState<number | "no" | null>(null);

  const refresh = useCallback(() => {
    if (!video || !ready || mediaKind !== "vod" || !mediaKey) {
      setAudioTracks([]);
      setSubTracks([]);
      setActiveAid(null);
      setActiveSid(null);
      return;
    }

    const { tracks: audio, active: aid } = readVideoAudioTracks(video);
    const { tracks: subs, active: sid } = readVideoTextTracks(video);
    setAudioTracks(audio);
    setSubTracks(subs);
    setActiveAid(aid);
    setActiveSid(sid);
  }, [video, ready, mediaKind, mediaKey]);

  useEffect(() => {
    if (!mediaKey || mediaKind !== "vod") {
      setAudioTracks([]);
      setSubTracks([]);
      setActiveAid(null);
      setActiveSid(null);
      return;
    }

    if (!video) return;

    const onChange = () => refresh();
    video.addEventListener("loadedmetadata", onChange);
    video.addEventListener("loadeddata", onChange);
    video.addEventListener("canplay", onChange);
    video.addEventListener("canplaythrough", onChange);
    video.textTracks.addEventListener("addtrack", onChange);
    video.textTracks.addEventListener("change", onChange);
    video.textTracks.addEventListener("removetrack", onChange);
    video.audioTracks?.addEventListener("addtrack", onChange);
    video.audioTracks?.addEventListener("change", onChange);
    video.audioTracks?.addEventListener("removetrack", onChange);

    refresh();
    const interval = window.setInterval(refresh, TRACK_PROBE_INTERVAL_MS);

    return () => {
      video.removeEventListener("loadedmetadata", onChange);
      video.removeEventListener("loadeddata", onChange);
      video.removeEventListener("canplay", onChange);
      video.removeEventListener("canplaythrough", onChange);
      video.textTracks.removeEventListener("addtrack", onChange);
      video.textTracks.removeEventListener("change", onChange);
      video.textTracks.removeEventListener("removetrack", onChange);
      video.audioTracks?.removeEventListener("addtrack", onChange);
      video.audioTracks?.removeEventListener("change", onChange);
      video.audioTracks?.removeEventListener("removetrack", onChange);
      window.clearInterval(interval);
    };
  }, [mediaKey, mediaKind, video, refresh]);

  const selectAudio = useCallback(
    (id: number) => {
      if (!video) return;
      applyVideoAudioTrack(video, id);
      setActiveAid(id);
      refresh();
    },
    [video, refresh],
  );

  const selectSubtitle = useCallback(
    (id: number | "no") => {
      if (!video) return;
      applyVideoSubtitleTrack(video, id);
      setActiveSid(id);
      refresh();
    },
    [video, refresh],
  );

  return {
    audioTracks,
    subTracks,
    activeAid,
    activeSid,
    loading: false,
    selectAudio,
    selectSubtitle,
    hasAudioTracks: audioTracks.length > 1,
    hasSubTracks: subTracks.length > 0,
  };
}
