export interface MpvTrack {
  id: number;
  title: string;
  lang?: string;
  selected: boolean;
}

interface RawMpvTrack {
  id?: number;
  type?: string;
  title?: string;
  lang?: string;
  selected?: boolean;
}

function langLabel(lang?: string): string | undefined {
  if (!lang) return undefined;
  const normalized = lang.trim().toLowerCase();
  const map: Record<string, string> = {
    por: "Português",
    pt: "Português",
    "pt-br": "Português",
    "pt_br": "Português",
    eng: "Inglês",
    en: "Inglês",
    "en-us": "Inglês",
    spa: "Espanhol",
    es: "Espanhol",
  };
  return map[normalized] ?? lang;
}

function trackLabel(track: RawMpvTrack, index: number, kind: "audio" | "sub"): string {
  const title = track.title?.trim();
  const lang = langLabel(track.lang);
  if (title && lang && !title.toLowerCase().includes(lang.toLowerCase())) {
    return `${title} · ${lang}`;
  }
  if (title) return title;
  if (lang) return lang;
  return kind === "audio" ? `Áudio ${index + 1}` : `Legenda ${index + 1}`;
}

export function parseMpvTrackList(raw: unknown): {
  audio: MpvTrack[];
  sub: MpvTrack[];
} {
  if (!Array.isArray(raw)) {
    return { audio: [], sub: [] };
  }

  const audio: MpvTrack[] = [];
  const sub: MpvTrack[] = [];

  for (const entry of raw as RawMpvTrack[]) {
    if (typeof entry.id !== "number" || !entry.type) continue;
    const track: MpvTrack = {
      id: entry.id,
      title: trackLabel(entry, entry.type === "audio" ? audio.length : sub.length, entry.type === "audio" ? "audio" : "sub"),
      lang: entry.lang,
      selected: Boolean(entry.selected),
    };
    if (entry.type === "audio") audio.push(track);
    if (entry.type === "sub") sub.push(track);
  }

  return { audio, sub };
}
