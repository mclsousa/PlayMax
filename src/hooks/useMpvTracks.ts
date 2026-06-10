import { isTauri } from "@tauri-apps/api/core";

import { useCallback, useEffect, useState } from "react";

import { getProperty, setProperty } from "tauri-plugin-libmpv-api";

import { parseMpvTrackList, type MpvTrack } from "../lib/mpvTracks";

import { withMpvLock } from "../lib/mpvLock";

const TRACK_PROBE_DELAYS_MS = [500, 1500, 4000, 8000];

export function useMpvTracks(
  enabled: boolean,
  mediaKey: string | null,
  probeToken = 0,
) {
  const [audioTracks, setAudioTracks] = useState<MpvTrack[]>([]);
  const [subTracks, setSubTracks] = useState<MpvTrack[]>([]);
  const [activeAid, setActiveAid] = useState<number | null>(null);
  const [activeSid, setActiveSid] = useState<number | "no" | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!enabled || !isTauri() || !mediaKey) return;

    setLoading(true);

    try {
      await withMpvLock(async () => {
        const raw = await getProperty("track-list", "node");
        const { audio, sub } = parseMpvTrackList(raw);

        setAudioTracks(audio);
        setSubTracks(sub);

        const aid = await getProperty<number>("aid", "int64");
        setActiveAid(typeof aid === "number" ? aid : null);

        const sidRaw = await getProperty("sid", "string");
        if (sidRaw === "no" || sidRaw === "auto") {
          setActiveSid("no");
        } else {
          const sidNum = await getProperty<number>("sid", "int64");
          setActiveSid(typeof sidNum === "number" ? sidNum : null);
        }
      });
    } catch {
      setAudioTracks([]);
      setSubTracks([]);
    } finally {
      setLoading(false);
    }
  }, [enabled, mediaKey]);

  useEffect(() => {
    if (!mediaKey || !enabled) {
      setAudioTracks([]);
      setSubTracks([]);
      setActiveAid(null);
      setActiveSid(null);
      return;
    }

    void refresh();
    const timers = TRACK_PROBE_DELAYS_MS.map((delay) =>
      window.setTimeout(() => void refresh(), delay),
    );

    return () => {
      timers.forEach((timer) => window.clearTimeout(timer));
    };
  }, [mediaKey, enabled, probeToken, refresh]);

  const selectAudio = useCallback(
    async (id: number) => {
      if (!enabled || !isTauri()) return;
      await withMpvLock(async () => {
        await setProperty("aid", id);
        setActiveAid(id);
      });
      await refresh();
    },
    [enabled, refresh],
  );

  const selectSubtitle = useCallback(
    async (id: number | "no") => {
      if (!enabled || !isTauri()) return;
      await withMpvLock(async () => {
        if (id === "no") {
          await setProperty("sid", "no");
          setActiveSid("no");
        } else {
          await setProperty("sid", id);
          setActiveSid(id);
        }
      });
      await refresh();
    },
    [enabled, refresh],
  );

  return {
    audioTracks,
    subTracks,
    activeAid,
    activeSid,
    loading,
    refresh,
    selectAudio,
    selectSubtitle,
    hasAudioTracks: audioTracks.length > 1,
    hasSubTracks: subTracks.length > 0,
  };
}
