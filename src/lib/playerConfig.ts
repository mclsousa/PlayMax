/** User-Agent aceito pela maioria dos provedores IPTV */
export const IPTV_USER_AGENT =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

export const MPV_INITIAL_OPTIONS: Record<string, string> = {
  // OpenGL via ANGLE avoids D3D11 swapchain crashes when embedding into WebView2.
  vo: "gpu",
  "gpu-api": "opengl",
  "gpu-context": "angle",
  profile: "fast",
  scale: "bilinear",
  dither: "no",
  hwdec: "no",
  "keep-open": "yes",
  // Defer native video window until a file is loaded (less GPU churn at init).
  "force-window": "no",
  cache: "yes",
  "demuxer-max-bytes": "150MiB",
  "network-timeout": "20",
  "user-agent": IPTV_USER_AGENT,
  "stream-lavf-o": "reconnect=1,reconnect_streamed=1,reconnect_delay_max=5",
  "load-scripts": "no",
};

export function buildLoadfileArgs(streamUrl: string): (string | number)[] {
  let referrer = "";
  try {
    referrer = `${new URL(streamUrl).origin}/`;
  } catch {
    referrer = "";
  }

  const options = referrer
    ? `user-agent="${IPTV_USER_AGENT}",referrer="${referrer}"`
    : `user-agent="${IPTV_USER_AGENT}"`;

  return [streamUrl, "replace", -1, options];
}

export function describeMpvEndReason(reason: string, errorCode: number): string {
  if (reason === "error") {
    return `Stream encerrou com erro (código ${errorCode}). Verifique se o canal está online.`;
  }
  if (reason === "eof") {
    return "Stream encerrado inesperadamente (EOF).";
  }
  return `Falha na reprodução (${reason}).`;
}
