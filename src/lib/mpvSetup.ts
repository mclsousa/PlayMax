import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";

/** WebView2 needs an explicit transparent background for mpv video to show through. */
export async function setupTransparentWebview(): Promise<void> {
  if (!isTauri()) return;
  try {
    await getCurrentWebview().setBackgroundColor({
      red: 0,
      green: 0,
      blue: 0,
      alpha: 0,
    });
  } catch {
    // Preview web / unsupported platform
  }
}
