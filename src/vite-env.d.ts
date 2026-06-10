/// <reference types="vite/client" />

interface AudioTrack {
  enabled: boolean;
  readonly id: string;
  readonly kind: string;
  readonly label: string;
  readonly language: string;
  readonly sourceBuffer: SourceBuffer | null;
}

interface AudioTrackList extends EventTarget {
  readonly length: number;
  [index: number]: AudioTrack;
}

interface HTMLVideoElement {
  readonly audioTracks: AudioTrackList;
}
