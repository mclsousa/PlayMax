const LOW_RES_PATH_REPLACEMENTS: [RegExp, string][] = [
  [/\/thumb(?:nail)?s?\//i, "/original/"],
  [/\/small\//i, "/large/"],
  [/\/sm\//i, "/large/"],
  [/\/icon\//i, "/original/"],
  [/\/icons\//i, "/original/"],
  [/\/w\d{1,3}\//i, "/original/"],
  [/\/s\d{1,3}x\d{1,3}\//i, "/original/"],
];

const LOW_RES_QUERY_REPLACEMENTS: [RegExp, string][] = [
  [/(?<=[?&])size=(?:small|thumb|thumbnail|icon)(?=&|$)/i, "size=large"],
  [/(?<=[?&])w=\d{1,3}(?=&|$)/i, "w=original"],
];

const LOW_RES_FILENAME_REPLACEMENTS: [RegExp, string][] = [
  [/_small(\.[a-z0-9]{2,4})(?=$|\?)/i, "_big$1"],
  [/-small(\.[a-z0-9]{2,4})(?=$|\?)/i, "-big$1"],
  [/_thumb(?:nail)?(\.[a-z0-9]{2,4})(?=$|\?)/i, "_big$1"],
];

const TMDB_SIZE_SEGMENT = /\/t\/p\/(?!original\/)[^/]+\//i;

export type PosterSize = "thumb" | "card" | "hero";

function tmdbSizeSegment(size: PosterSize): string {
  switch (size) {
    case "thumb":
      return "/t/p/w342/";
    case "card":
      return "/t/p/w780/";
    case "hero":
      return "/t/p/original/";
  }
}

export function upscalePosterUrl(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) return trimmed;

  let result = trimmed;
  for (const [pattern, replacement] of LOW_RES_PATH_REPLACEMENTS) {
    if (pattern.test(result)) {
      result = result.replace(pattern, replacement);
      break;
    }
  }

  for (const [pattern, replacement] of LOW_RES_FILENAME_REPLACEMENTS) {
    if (pattern.test(result)) {
      result = result.replace(pattern, replacement);
      break;
    }
  }

  if (TMDB_SIZE_SEGMENT.test(result)) {
    result = result.replace(TMDB_SIZE_SEGMENT, "/t/p/original/");
  }

  for (const [pattern, replacement] of LOW_RES_QUERY_REPLACEMENTS) {
    if (pattern.test(result)) {
      result = result.replace(pattern, replacement);
    }
  }

  return result;
}

export function posterUrlForSize(
  poster?: string | null,
  backdrop?: string | null,
  size: PosterSize = "card",
): string | null {
  const raw = poster?.trim() || backdrop?.trim() || null;
  if (!raw) return null;

  if (size === "hero") {
    return upscalePosterUrl(raw);
  }

  if (TMDB_SIZE_SEGMENT.test(raw)) {
    return raw.replace(TMDB_SIZE_SEGMENT, tmdbSizeSegment(size));
  }

  if (size === "thumb") {
    return raw;
  }

  return upscalePosterUrl(raw);
}

// w1280 loads ~10x faster than `original` (multi-MB) and is sharp enough for
// full-width hero areas; `original` made backdrops hang on slow connections.
const TMDB_ANY_SIZE_SEGMENT = /\/t\/p\/[^/]+\//i;

export function heroBackdropUrl(backdrop?: string | null): string | null {
  const raw = backdrop?.trim();
  if (!raw) return null;

  if (TMDB_ANY_SIZE_SEGMENT.test(raw)) {
    return raw.replace(TMDB_ANY_SIZE_SEGMENT, "/t/p/w1280/");
  }

  return upscalePosterUrl(raw);
}

export function bestPosterUrl(
  poster?: string | null,
  backdrop?: string | null,
): string | null {
  return posterUrlForSize(poster, backdrop, "hero");
}

export function cardPosterUrl(
  poster?: string | null,
  backdrop?: string | null,
): string | null {
  return posterUrlForSize(poster, backdrop, "card");
}

export function thumbPosterUrl(
  poster?: string | null,
  backdrop?: string | null,
): string | null {
  return posterUrlForSize(poster, backdrop, "thumb");
}
