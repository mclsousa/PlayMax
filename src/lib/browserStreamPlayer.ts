import type { default as MpegtsDefault } from "mpegts.js";
import { IPTV_USER_AGENT } from "./playerConfig";

type MpegtsModule = typeof MpegtsDefault;
type MpegtsPlayer = ReturnType<MpegtsModule["createPlayer"]>;

let mpegtsModulePromise: Promise<MpegtsModule> | null = null;

async function loadMpegtsModule(): Promise<MpegtsModule> {
  if (!mpegtsModulePromise) {
    mpegtsModulePromise = import("mpegts.js").then((module) => module.default);
  }
  return mpegtsModulePromise;
}

function streamHeaders(url: string): Record<string, string> {
  const headers: Record<string, string> = {
    "User-Agent": IPTV_USER_AGENT,
  };
  try {
    headers.Referer = `${new URL(url).origin}/`;
  } catch {
    // ignore invalid URLs
  }
  return headers;
}

function isNativeProgressiveUrl(url: string): boolean {
  const lower = url.toLowerCase().split("?")[0] ?? url;
  return (
    lower.endsWith(".mp4") ||
    lower.endsWith(".webm") ||
    lower.endsWith(".m3u8") ||
    lower.includes(".mp4?") ||
    lower.includes(".m3u8?")
  );
}

export class BrowserStreamPlayer {
  private mpegtsPlayer: MpegtsPlayer | null = null;

  detach(video: HTMLVideoElement): void {
    if (this.mpegtsPlayer) {
      try {
        this.mpegtsPlayer.pause();
        this.mpegtsPlayer.unload();
        this.mpegtsPlayer.detachMediaElement();
        this.mpegtsPlayer.destroy();
      } catch {
        // Best-effort teardown.
      }
      this.mpegtsPlayer = null;
    }
    video.pause();
    video.removeAttribute("src");
    video.load();
  }

  async play(video: HTMLVideoElement): Promise<void> {
    if (this.mpegtsPlayer) {
      await this.mpegtsPlayer.play();
      return;
    }
    await video.play();
  }

  pause(video: HTMLVideoElement): void {
    if (this.mpegtsPlayer) {
      this.mpegtsPlayer.pause();
      return;
    }
    video.pause();
  }

  async load(
    video: HTMLVideoElement,
    url: string,
    kind: "live" | "vod",
  ): Promise<void> {
    this.detach(video);

    const isLive = kind === "live";

    if (!isLive && isNativeProgressiveUrl(url)) {
      video.src = url;
      await this.play(video);
      return;
    }

    const mpegts = await loadMpegtsModule();

    if (!mpegts.getFeatureList().mseLivePlayback) {
      throw new Error("Seu ambiente não suporta reprodução MSE para IPTV.");
    }

    const player = mpegts.createPlayer(
      {
        type: "mpegts",
        isLive,
        url,
        hasAudio: true,
        hasVideo: true,
      },
      {
        enableWorker: true,
        lazyLoad: !isLive,
        lazyLoadMaxDuration: isLive ? 90 : 180,
        stashInitialSize: 128,
        headers: streamHeaders(url),
      },
    );

    player.attachMediaElement(video);
    player.load();
    this.mpegtsPlayer = player;
    await this.play(video);
  }
}
