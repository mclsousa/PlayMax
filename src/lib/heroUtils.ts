import type { CatalogItem, Movie } from "./types";

const TMDB_IMAGE_HOST = "image.tmdb.org";
const MIN_HERO_SYNOPSIS_LENGTH = 40;

const CHANNEL_MARKERS = [
  "24H",
  "24 HORAS",
  "24 HRS",
  "24HRS",
  "AO VIVO",
  "LIVE TV",
  " LIVE ",
  " CANAL ",
  " CANAIS ",
  " PREMIUM TV",
  " ESPORTES ",
  " PPV ",
  " PAY PER VIEW ",
] as const;

const STRONG_CHANNEL_BRANDS = [
  "MEGAPIX",
  "MEGA PIX",
  "PREMIERE",
  "TELECINE",
  "TELE CINE",
  "SPORTV",
  "SPORT TV",
  "FOX SPORTS",
  "MULTISHOW",
] as const;

const WEAK_CHANNEL_BRANDS = [
  "PREMIUM",
  "HBO",
  "DISCOVERY",
  "ANIMAL PLANET",
  "NATIONAL GEOGRAPHIC",
  "WARNER",
  "TNT",
  "SPACE",
  "AXN",
  "FOX",
  "GLOBO",
  "RECORD",
  "SBT",
  "BAND",
  "CNN",
  "ESPN",
  "PARAMOUNT",
  "UNIVERSAL",
  "SONY",
  "CINEMAX",
  "STARZ",
  "SHOWTIME",
  "DISNEY",
  "NAT GEO",
  "HISTORY",
  "LIFETIME",
  "COMEDY",
  "GNT",
  "VIVA",
  "OFF",
  "BIS",
] as const;

export function looksLikeChannelEntry(item: CatalogItem): boolean {
  const name = item.name.trim();
  if (!name) {
    return true;
  }

  const upper = name.toUpperCase();

  if (CHANNEL_MARKERS.some((marker) => upper.includes(marker))) {
    return true;
  }

  if (STRONG_CHANNEL_BRANDS.some((brand) => upper.includes(brand))) {
    return true;
  }

  const hasQuality =
    /\b(FHD|4K|UHD|HD)\b/.test(upper) || upper.includes(" HD") || upper.startsWith("HD ");
  const hasChannelWord =
    upper.includes("PREMIUM") ||
    upper.includes("LIVE") ||
    upper.includes(" TV") ||
    upper.includes("CANAL") ||
    upper.includes("TC ");

  if (hasQuality && hasChannelWord) {
    return true;
  }

  if (
    hasQuality &&
    WEAK_CHANNEL_BRANDS.some((brand) => upper.includes(brand))
  ) {
    return true;
  }

  if (
    WEAK_CHANNEL_BRANDS.some((brand) => upper.includes(brand)) &&
    (upper.includes(" TV") || upper.includes("CANAL") || name.length <= 18)
  ) {
    return true;
  }

  if (upper.includes("PREMIUM") && upper.includes("TC")) {
    return true;
  }

  if (
    /^(TC|TV|CANAL)\b/i.test(name) ||
    upper === "TC" ||
    upper === "TV"
  ) {
    return true;
  }

  if (/\b(FHD|4K|UHD|HD)\s*$/i.test(name) && name.length <= 40) {
    return true;
  }

  if (/^[A-Z0-9\s+.\-]+(FHD|4K|UHD|HD)$/i.test(name)) {
    return true;
  }

  return false;
}

function heroItemScore(item: CatalogItem): number {
  let score = 0;

  if (item.backdrop?.trim()) {
    score += 30;
  } else if (item.poster?.trim()) {
    score += 5;
  }

  return score + item.addedAt;
}

function isHeroCandidate(item: CatalogItem): boolean {
  if (item.itemType !== "movie") {
    return false;
  }

  if (!item.backdrop?.trim()) {
    return false;
  }

  if (looksLikeChannelEntry(item)) {
    return false;
  }

  return true;
}

function pickBestHero(items: CatalogItem[]): CatalogItem | null {
  const candidates = items.filter(isHeroCandidate);
  if (candidates.length === 0) {
    return null;
  }

  return [...candidates].sort((a, b) => heroItemScore(b) - heroItemScore(a))[0];
}

function pickCatalogFallback(
  releases: CatalogItem[],
  recentMovies: CatalogItem[],
  recentSeries: CatalogItem[],
): CatalogItem | null {
  const pools = [releases, recentMovies, recentSeries];
  for (const pool of pools) {
    const withArt = pool.find(
      (item) => item.poster?.trim() || item.backdrop?.trim(),
    );
    if (withArt) {
      return withArt;
    }
  }
  for (const pool of pools) {
    if (pool.length > 0) {
      return pool[0];
    }
  }
  return null;
}

export function pickHeroItem(
  releases: CatalogItem[],
  recentMovies: CatalogItem[],
  recentSeries: CatalogItem[],
): CatalogItem | null {
  return (
    pickBestHero(releases) ??
    pickBestHero(recentMovies) ??
    pickCatalogFallback(releases, recentMovies, recentSeries)
  );
}

export function isTmdbBackdrop(url?: string | null): boolean {
  return Boolean(url?.trim().includes(TMDB_IMAGE_HOST));
}

export function isQualifiedHeroMovie(movie: Movie): boolean {
  const plot = movie.plot?.trim() ?? "";
  if (plot.length < MIN_HERO_SYNOPSIS_LENGTH) {
    return false;
  }

  if (!isTmdbBackdrop(movie.backdrop)) {
    return false;
  }

  return true;
}
