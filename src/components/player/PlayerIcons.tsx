import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const defaults = {
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

export function PlayIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M8 5v14l11-7z" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function PauseIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M7 5h3v14H7zM14 5h3v14h-3z" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function StopIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <rect x="6" y="6" width="12" height="12" rx="1.5" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function ReloadIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M3 12a9 9 0 0 1 15.5-6.4" />
      <path d="M21 3v5h-5" />
      <path d="M21 12a9 9 0 0 1-15.5 6.4" />
      <path d="M3 21v-5h5" />
    </svg>
  );
}

export function VolumeIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M11 5 6 9H3v6h3l5 4V5z" />
      <path d="M15.5 8.5a5 5 0 0 1 0 7" />
      <path d="M18 6a8 8 0 0 1 0 12" />
    </svg>
  );
}

export function VolumeMutedIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M11 5 6 9H3v6h3l5 4V5z" />
      <path d="m16 9 5 6M21 9l-5 6" />
    </svg>
  );
}

export function FullscreenIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5" />
    </svg>
  );
}

export function FullscreenExitIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M9 4H4v5M20 9V4h-5M15 20h5v-5M4 15v5h5" />
    </svg>
  );
}

export function SubtitlesIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <rect x="3" y="5" width="18" height="14" rx="2" />
      <path d="M7 14h4M13 14h4M7 17h6" />
    </svg>
  );
}

export function AudioTrackIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M4 10v4" />
      <path d="M7 8v8" />
      <path d="M10 6v12" />
      <path d="M13 9v6" />
      <path d="M16 7v10" />
      <path d="M19 10v4" />
    </svg>
  );
}

export function SkipPrevIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M6 6v12" />
      <path d="M18 6 10 12l8 6z" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function SkipNextIcon(props: IconProps) {
  return (
    <svg {...defaults} {...props}>
      <path d="M6 6l8 6-8 6z" fill="currentColor" stroke="none" />
      <path d="M18 6v12" />
    </svg>
  );
}
