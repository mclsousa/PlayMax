import type { VodPlayInfo } from "./types";

/** Route where VOD playback belongs — never `/live`. */
export function vodPlayPath(info: Pick<VodPlayInfo, "itemType">): "/movies" | "/series" {
  return info.itemType === "episode" ? "/series" : "/movies";
}

export interface VodPlayLocationState {
  vod: VodPlayInfo;
  category?: string;
  seriesId?: string;
}
