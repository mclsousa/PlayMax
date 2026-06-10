import type { MpvTrack } from "./mpvTracks";

function textTrackLabel(track: TextTrack, index: number): string {
  const parts = [track.label, track.language].filter(Boolean);
  if (parts.length > 0) return parts.join(" · ");
  return `Legenda ${index + 1}`;
}

function audioTrackLabel(track: AudioTrack, index: number): string {
  const parts = [track.label, track.language].filter(Boolean);
  if (parts.length > 0) return parts.join(" · ");
  return `Áudio ${index + 1}`;
}

function isSubtitleTrack(track: TextTrack): boolean {
  return track.kind === "subtitles" || track.kind === "captions";
}

export function readVideoTextTracks(video: HTMLVideoElement): {
  tracks: MpvTrack[];
  active: number | "no" | null;
} {
  const tracks: MpvTrack[] = [];
  let active: number | "no" | null = "no";
  const list = video.textTracks;

  for (let i = 0; i < list.length; i++) {
    const track = list[i];
    if (!isSubtitleTrack(track)) continue;
    tracks.push({
      id: i,
      title: textTrackLabel(track, tracks.length),
      lang: track.language || undefined,
      selected: track.mode === "showing",
    });
    if (track.mode === "showing") active = i;
  }

  return { tracks, active };
}

export function readVideoAudioTracks(video: HTMLVideoElement): {
  tracks: MpvTrack[];
  active: number | null;
} {
  const tracks: MpvTrack[] = [];
  let active: number | null = null;
  const list = video.audioTracks;
  if (!list) return { tracks, active: null };

  for (let i = 0; i < list.length; i++) {
    const track = list[i];
    tracks.push({
      id: i,
      title: audioTrackLabel(track, i),
      lang: track.language || undefined,
      selected: track.enabled,
    });
    if (track.enabled) active = i;
  }

  return { tracks, active };
}

export function applyVideoAudioTrack(video: HTMLVideoElement, id: number): void {
  const list = video.audioTracks;
  if (!list) return;
  for (let i = 0; i < list.length; i++) {
    list[i].enabled = i === id;
  }
}

export function applyVideoSubtitleTrack(
  video: HTMLVideoElement,
  id: number | "no",
): void {
  for (let i = 0; i < video.textTracks.length; i++) {
    const track = video.textTracks[i];
    if (!isSubtitleTrack(track)) continue;
    track.mode = "disabled";
  }
  if (id !== "no") {
    const track = video.textTracks[id];
    if (track && isSubtitleTrack(track)) {
      track.mode = "showing";
    }
  }
}
