//! Unified content classification for M3U entries and catalog query filters.

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    LiveChannel,
    Movie,
    Series,
}

static MOVIE_YEAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:\(|\[|\s)(?:19|20)\d{2}(?:\)|\]|\s|$)").unwrap());

static DURATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b\d{1,3}\s*(?:min(?:utos?)?|mins?|h(?:oras?)?|hr?s?)\b").unwrap()
});

static SERIES_EPISODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\bS\d{1,2}\s*E\d{1,3}\b|\b\d{1,2}x\d{1,3}\b)").unwrap()
});

const CHANNEL_MARKERS: &[&str] = &[
    "24H", "24 HORAS", "24 HRS", "24HRS", "AO VIVO", "LIVE TV", " LIVE ", " CANAL ",
    " CANAIS ", " PREMIUM TV", " ESPORTES ", " PPV ", " PAY PER VIEW ",
];

const QUALITY_SUFFIXES: &[&str] = &[
    "FHD", "HD", "SD", "4K", "UHD", "H265", "H264", "HEVC", "BR", "1080P", "720P", "576P",
];

/// Longer bases first so `H265` is not mistaken for `HD` + `265`.
const QUALITY_BASES: &[&str] = &["FHD", "UHD", "H265", "H264", "HEVC", "1080P", "720P", "576P", "4K", "SD", "HD"];

const SINGLE_WORD_BRANDS: &[&str] = &[
    "MEGAPIX", "PREMIERE", "TELECINE", "HBO", "SPORTV", "ESPN", "BAND", "SBT", "GLOBO",
    "RECORD", "DISCOVERY", "NICK", "CARTOON", "TCM", "AXN", "WARNER", "FOX", "SPACE",
    "MULTISHOW", "GNT", "VIVA", "OFF", "SONY", "CNN", "TNT", "FX", "AMC", "PARAMOUNT",
    "DISNEY", "MTV", "E!", "LIFETIME", "HISTORY", "COMBATE", "CANAL", "TC", "UNIVERSAL",
    "USA",
];

const TC_CHANNEL_VARIANTS: &[&str] = &[
    "ACTION", "PIPOCA", "FUN", "TOUCH", "CULT", "PREMIUM", "PULSE",
];

const MULTI_WORD_BRANDS: &[&str] = &[
    "UNIVERSAL TV", "UNIVERSAL PREMIER", "UNIVERSAL PREMIUM", "UNIVERSAL PREMIERE",
    "STUDIO UNIVERSAL", "SPORT TV", "NICKELODEON", "NICK JR",
    "COMEDY CENTRAL", "NAT GEO", "ANIMAL PLANET", "DISCOVERY CHANNEL", "DISCOVERY KIDS",
    "DISCOVERY HOME", "DISCOVERY WORLD", "DISCOVERY SCIENCE", "DISCOVERY THEATER",
    "ESPN BRASIL", "ESPN 2", "ESPN 3", "ESPN 4", "ESPN EXTRA", "HBO 2", "HBO PLUS",
    "HBO POP", "HBO SIGNATURE", "HBO MAX", "HBO MUNDI", "TELECINE PREMIUM",
    "TELECINE ACTION", "TELECINE FUN", "TELECINE PIPOCA", "TELECINE TOUCH",
    "TELECINE CULT", "TC ACTION", "TC PIPOCA", "TC FUN", "TC TOUCH", "TC CULT",
    "TC PREMIUM", "TC PREMIERE", "USA CHANNEL", "USA TV",
    "PREMIERE CLUBES", "PREMIERE 2", "PREMIERE 3", "PREMIERE 4",
    "PREMIERE 5", "PREMIERE 6", "PREMIERE 7", "GLOBO NEWS", "GLOBOPLAY",
    "RECORD NEWS", "BAND NEWS", "BAND SPORTS", "CARTOON NETWORK", "WARNER CHANNEL",
    "WARNER TV", "FOX SPORTS", "FOX NEWS", "FOX LIFE", "SONY CHANNEL", "SONY MOVIES",
    "LIFETIME MOVIES", "PARAMOUNT NETWORK", "PARAMOUNT CHANNEL",
];

const AMBIGUOUS_CAPS_ONLY_BRANDS: &[&str] =
    &["FOX", "SPACE", "OFF", "FX", "TNT", "CNN", "TC", "AXN", "GNT", "VIVA", "MTV"];

pub fn has_movie_year(name: &str) -> bool {
    MOVIE_YEAR_RE.is_match(name)
}

static RELEASE_YEAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\[|\(|\s)((?:19|20)\d{2})(?:\]|\)|\s|$)").unwrap());

/// Extracts the latest 4-digit release year embedded in a title, e.g. the
/// `2026` in "A Dama [2026]" or "Vikings (2013)". Returns None when absent.
/// Used to tell "Lançamentos" (new releases) from older recently-added titles.
pub fn latest_release_year(name: &str) -> Option<i32> {
    RELEASE_YEAR_RE
        .captures_iter(name)
        .filter_map(|caps| caps.get(1)?.as_str().parse::<i32>().ok())
        .max()
}

/// Normalizes an IPTV token: uppercase, maps modifier-letter caps (ᴮᴿ), strips `*`.
pub fn normalize_iptv_token(token: &str) -> String {
    token
        .to_uppercase()
        .chars()
        .map(|c| match c {
            'ᴮ' => 'B',
            'ᴿ' => 'R',
            'ᴴ' => 'H',
            'ᴰ' => 'D',
            _ => c,
        })
        .collect::<String>()
        .trim_end_matches('*')
        .to_string()
}

/// True when a token is an IPTV quality/resolution marker, including decorated
/// variants like `FHD*`, `HDᴮᴿ`, and `SD BR`.
pub fn is_quality_token(token: &str) -> bool {
    let norm = normalize_iptv_token(token);
    if norm.is_empty() {
        return false;
    }
    if QUALITY_SUFFIXES.iter().any(|suffix| norm == *suffix) {
        return true;
    }
    for base in QUALITY_BASES {
        if norm == *base {
            return true;
        }
        if norm.starts_with(base) {
            let rest = &norm[base.len()..];
            if rest.is_empty() || rest.chars().all(|c| matches!(c, 'B' | 'R')) {
                return true;
            }
        }
    }
    false
}

/// Strips trailing IPTV quality tokens (FHD, HD, 4K, etc.) for title validation.
pub fn title_without_quality_suffix(name: &str) -> String {
    let mut tokens: Vec<&str> = name.trim().split_whitespace().collect();
    while tokens.last().is_some_and(|t| is_quality_token(t)) {
        tokens.pop();
    }
    tokens.join(" ")
}

pub fn has_duration_in_name(name: &str) -> bool {
    DURATION_RE.is_match(name)
}

pub fn looks_like_series_entry(name: &str) -> bool {
    SERIES_EPISODE_RE.is_match(name.trim())
}

pub fn is_channel_brand(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    let upper = trimmed.to_uppercase();
    matches_iptv_brand(&upper, trimmed)
}

pub fn looks_like_movie_title(name: &str) -> bool {
    if is_channel_like_name(name) {
        return false;
    }
    has_movie_year(name) || has_duration_in_name(name)
}

pub fn classify_m3u_entry(name: &str, group: &str, has_episode_pattern: bool) -> ContentKind {
    if is_channel_like_name(name) {
        return ContentKind::LiveChannel;
    }

    if has_episode_pattern || looks_like_series_entry(name) {
        return ContentKind::Series;
    }

    let lower = group.to_lowercase();

    if is_live_group(&lower) {
        return ContentKind::LiveChannel;
    }

    if is_series_group_pattern(&lower) {
        return ContentKind::Series;
    }

    if is_explicit_movie_group(&lower) {
        return ContentKind::Movie;
    }

    if is_genre_group(&lower) {
        if looks_like_movie_title(name) {
            return ContentKind::Movie;
        }
        return ContentKind::LiveChannel;
    }

    ContentKind::LiveChannel
}

pub fn classify_group(group: &str) -> ContentKind {
    let lower = group.to_lowercase();

    if is_live_group(&lower) {
        return ContentKind::LiveChannel;
    }

    if is_series_group_pattern(&lower) {
        return ContentKind::Series;
    }

    if is_explicit_movie_group(&lower) {
        return ContentKind::Movie;
    }

    if is_genre_group(&lower) {
        return ContentKind::LiveChannel;
    }

    ContentKind::LiveChannel
}

pub fn is_channel_like_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return true;
    }

    let upper = trimmed.to_uppercase();

    if CHANNEL_MARKERS.iter().any(|marker| upper.contains(marker)) {
        return true;
    }

    if is_channel_brand(trimmed) {
        return true;
    }

    if ends_with_quality_suffix(&upper) && !has_movie_year(trimmed) && tokens_are_short(&upper) {
        return true;
    }

    let has_quality = upper.contains("FHD")
        || upper.contains("4K")
        || upper.contains("UHD")
        || upper.contains(" HD")
        || upper.starts_with("HD ");
    let has_channel_word = upper.contains("PREMIUM")
        || upper.contains("LIVE")
        || upper.contains(" TV")
        || upper.contains("CANAL");
    if has_quality && has_channel_word {
        return true;
    }

    if upper.contains("PREMIUM") && upper.contains("TC") {
        return true;
    }

    if upper.starts_with("TC ")
        || upper.starts_with("TV ")
        || upper.starts_with("CANAL ")
        || upper == "TC"
        || upper == "TV"
    {
        return true;
    }

    false
}

fn is_all_caps_iptv_style(name: &str) -> bool {
    name.trim()
        .chars()
        .filter(|c| c.is_alphabetic())
        .all(|c| !c.is_lowercase())
}

fn is_quality_or_tv_variant(token: &str) -> bool {
    is_quality_token(token)
        || matches!(
            normalize_iptv_token(token).as_str(),
            "TV" | "CHANNEL" | "CH" | "PLUS" | "PREMIER" | "PREMIUM" | "PREMIERE" | "2" | "3"
                | "4" | "5" | "6" | "7"
        )
}

fn ends_with_quality_suffix(upper: &str) -> bool {
    upper
        .split_whitespace()
        .last()
        .is_some_and(is_quality_token)
}

fn is_tc_channel_variant(tokens: &[&str]) -> bool {
    if tokens.len() < 2 || tokens[0] != "TC" {
        return false;
    }
    TC_CHANNEL_VARIANTS.contains(&tokens[1])
}

fn is_usa_channel(tokens: &[&str], original: &str) -> bool {
    if tokens.len() > 4 || tokens.is_empty() || tokens[0] != "USA" {
        return false;
    }
    if has_movie_year(original) {
        return false;
    }
    tokens.len() == 1
        || tokens[1..]
            .iter()
            .all(|token| is_quality_or_tv_variant(token))
}

fn is_universal_channel(tokens: &[&str], original: &str) -> bool {
    if tokens.is_empty() || tokens[0] != "UNIVERSAL" {
        return false;
    }
    if has_movie_year(original) {
        return false;
    }
    tokens.len() == 1
        || tokens[1..]
            .iter()
            .all(|token| is_quality_or_tv_variant(token))
}

fn matches_iptv_brand(upper: &str, original: &str) -> bool {
    let tokens: Vec<&str> = upper.split_whitespace().filter(|t| !t.is_empty()).collect();
    if tokens.is_empty() {
        return false;
    }

    if is_tc_channel_variant(&tokens)
        || is_usa_channel(&tokens, original)
        || is_universal_channel(&tokens, original)
    {
        return true;
    }

    let joined = tokens.join(" ");

    for brand in MULTI_WORD_BRANDS {
        if joined == *brand || joined.starts_with(&format!("{brand} ")) {
            return true;
        }
    }

    if tokens.len() <= 4 {
        let first = tokens[0];
        let rest = &tokens[1..];

        if SINGLE_WORD_BRANDS.contains(&first) {
            let ambiguous = AMBIGUOUS_CAPS_ONLY_BRANDS.contains(&first);
            if ambiguous && !is_all_caps_iptv_style(original) {
                return false;
            }
            if rest.is_empty() || rest.iter().all(|t| is_quality_or_tv_variant(t)) {
                return true;
            }
        }
    }

    if upper.ends_with(" TV") && !has_movie_year(original) && tokens.len() <= 4 {
        return true;
    }

    false
}

fn tokens_are_short(upper: &str) -> bool {
    upper.split_whitespace().filter(|t| !t.is_empty()).count() <= 4
}

fn is_live_group(lower: &str) -> bool {
    const LIVE_PATTERNS: &[&str] = &[
        "canais", "channels", "live", "ao vivo", "24 horas", "24h", "24 hrs", "24hrs", "iptv",
        "esportes", "sports", "noticias", "notícias", "news", "variedades", "kids", "infantil",
        "religios", "religioso", "adult", "adulto", "ppv", "pay per view",
    ];
    LIVE_PATTERNS.iter().any(|pattern| lower.contains(pattern))
}

fn is_explicit_movie_group(lower: &str) -> bool {
    const MOVIE_PATTERNS: &[&str] = &[
        "filmes", "movies", "vod", "cine", "cinema", "4k movies", "lancamentos", "lançamentos",
        "nacionais", "legendados", "dublados",
    ];
    MOVIE_PATTERNS.iter().any(|pattern| lower.contains(pattern))
}

fn is_genre_group(lower: &str) -> bool {
    const GENRE_PATTERNS: &[&str] = &[
        "comedia", "comédia", "comedy", "drama", "acao", "ação", "action", "terror", "horror",
        "animacao", "animação", "animation", "documentario", "documentário", "documentary",
        "documentarios", "documentários", "romance", "suspense", "thriller", "fantasia",
        "fantasy", "ficcao", "ficção", "ficcao cientifica", "ficção científica", "sci-fi",
    ];
    GENRE_PATTERNS.iter().any(|pattern| lower.contains(pattern))
}

pub fn is_movie_group(group: &str) -> bool {
    let lower = group.to_lowercase();
    is_explicit_movie_group(&lower) || is_genre_group(&lower)
}

pub fn is_series_group(group: &str) -> bool {
    let lower = group.to_lowercase();
    is_series_group_pattern(&lower)
}

/// True when a channel group belongs on the live TV page (not VOD movie/series categories).
pub fn is_live_tv_category(group: &str) -> bool {
    !is_movie_group(group) && !is_series_group(group)
}

/// Narrow check: the group name explicitly references movies/series/VOD. Used to
/// keep such labels out of the live category list even when (mislabelled) live
/// channels sit under them, without over-filtering genre-named live channels
/// (e.g. "Documentários", "Infantis") the way `is_live_tv_category` would.
pub fn is_vod_labeled_group(group: &str) -> bool {
    let lower = group.to_lowercase();
    [
        "filme", "serie", "séri", "vod", "cinema", "lançament", "lancament",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_series_group_pattern(lower: &str) -> bool {
    const SERIES_PATTERNS: &[&str] = &["séries", "series", "tv shows", "novelas", "animes"];
    SERIES_PATTERNS.iter().any(|pattern| lower.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_without_quality_suffix_strips_trailing_tokens() {
        assert_eq!(title_without_quality_suffix("MEGAPIX FHD"), "MEGAPIX");
        assert_eq!(title_without_quality_suffix("MEGAPIX FHD*"), "MEGAPIX");
        assert_eq!(title_without_quality_suffix("USA HDᴮᴿ"), "USA");
        assert_eq!(title_without_quality_suffix("MEGAPIX BR"), "MEGAPIX");
        assert_eq!(title_without_quality_suffix("Inception (2010) HD"), "Inception (2010)");
        assert_eq!(title_without_quality_suffix("Dune: Part Two"), "Dune: Part Two");
    }

    #[test]
    fn decorated_quality_suffixes_are_channels() {
        for name in [
            "MEGAPIX FHD*",
            "MEGAPIX FHDᴮᴿ",
            "MEGAPIX HD*",
            "MEGAPIX BR",
            "MEGAPIX SD*",
            "USA FHD*",
            "USA HDᴮᴿ",
            "UNIVERSAL PREMIER",
            "HBO FHD*",
            "TELECINE ACTION FHD*",
        ] {
            assert!(
                is_channel_like_name(name),
                "{name} should be detected as channel-like"
            );
            assert!(
                !looks_like_movie_title(name),
                "{name} should not look like a movie title"
            );
        }
    }

    #[test]
    fn telecine_tc_variants_are_channels() {
        for name in [
            "TC ACTION",
            "TC PIPOCA FHD",
            "TC FUN HD",
            "TC TOUCH",
            "TC CULT 4K",
            "TC PREMIUM",
            "TELECINE ACTION",
            "USA",
            "USA HD",
        ] {
            assert!(
                is_channel_like_name(name),
                "{name} should be detected as channel-like"
            );
            assert_eq!(
                classify_m3u_entry(name, "Acao", false),
                ContentKind::LiveChannel,
                "{name} in genre group should be LiveChannel"
            );
        }
    }

    #[test]
    fn iptv_channel_brands_are_live_not_movies() {
        for name in ["MEGAPIX FHD", "UNIVERSAL TV", "STUDIO UNIVERSAL"] {
            assert!(
                is_channel_like_name(name),
                "{name} should be detected as channel-like"
            );
            assert_eq!(
                classify_m3u_entry(name, "Comedia", false),
                ContentKind::LiveChannel,
                "{name} in genre group should be LiveChannel"
            );
            assert_eq!(
                classify_m3u_entry(name, "Filmes", false),
                ContentKind::LiveChannel,
                "{name} in Filmes group should still be LiveChannel"
            );
        }
        assert!(is_channel_brand("MEGAPIX FHD"));
    }

    #[test]
    fn genre_group_with_year_is_movie() {
        assert_eq!(
            classify_m3u_entry("20 ANOS + JOVEM (2013)", "Comedia", false),
            ContentKind::Movie
        );
        assert_eq!(
            classify_m3u_entry("Deadpool (2016)", "Comedia", false),
            ContentKind::Movie
        );
    }

    #[test]
    fn episode_pattern_is_series() {
        assert_eq!(
            classify_m3u_entry("Breaking Bad S01E05", "Filmes", true),
            ContentKind::Series
        );
        assert!(looks_like_series_entry("Breaking Bad S01E05"));
        assert!(looks_like_series_entry("Breaking Bad 1x05"));
    }

    #[test]
    fn classifies_explicit_movie_groups() {
        assert_eq!(classify_group("Filmes"), ContentKind::Movie);
        assert_eq!(classify_group("Lançamentos"), ContentKind::Movie);
    }

    #[test]
    fn classifies_live_groups() {
        assert_eq!(classify_group("Canais"), ContentKind::LiveChannel);
        assert_eq!(classify_group("24 Horas"), ContentKind::LiveChannel);
    }

    #[test]
    fn classifies_series_groups() {
        assert_eq!(classify_group("Séries"), ContentKind::Series);
    }

    #[test]
    fn channel_brand_overrides_movie_group() {
        assert_eq!(
            classify_m3u_entry("STUDIO UNIVERSAL", "Filmes", false),
            ContentKind::LiveChannel
        );
    }

    #[test]
    fn looks_like_movie_title_rejects_channels() {
        assert!(looks_like_movie_title("Inception (2010)"));
        assert!(!looks_like_movie_title("MEGAPIX FHD"));
    }

    #[test]
    fn live_tv_category_excludes_vod_groups() {
        for group in [
            "Comedia",
            "Drama",
            "Acao",
            "Lançamentos",
            "Filmes",
            "VOD",
            "Séries",
            "Novelas",
        ] {
            assert!(
                !is_live_tv_category(group),
                "{group} should not appear on live page"
            );
        }
        for group in ["Canais", "Esportes", "24 Horas", "Notícias", "Premium TV"] {
            assert!(
                is_live_tv_category(group),
                "{group} should appear on live page"
            );
        }
    }

    #[test]
    fn vod_labeled_group_flags_movie_series_names_only() {
        for g in ["GLOBOSAT FILMES", "FILMES E SERIES", "Cinema", "VOD 4K", "Lançamentos"] {
            assert!(is_vod_labeled_group(g), "{g} should be flagged as VOD-labelled");
        }
        for g in ["GLOBOS | SUDESTE", "ESPORTES", "DOCUMENTÁRIOS", "INFANTIS", "NOTICIAS"] {
            assert!(!is_vod_labeled_group(g), "{g} must stay on the live page");
        }
    }

    #[test]
    fn is_movie_group_covers_genre_and_explicit() {
        assert!(is_movie_group("Comedia"));
        assert!(is_movie_group("Lançamentos"));
        assert!(!is_movie_group("Canais"));
    }
}
