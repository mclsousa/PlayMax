use crate::db::models::{CastMember, Movie, Series};
use crate::error::AppResult;
use regex::Regex;
use std::sync::LazyLock;

const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/w780";
const TMDB_BACKDROP_BASE: &str = "https://image.tmdb.org/t/p/original";
const TMDB_PROFILE_BASE: &str = "https://image.tmdb.org/t/p/w185";
const TMDB_API_KEY: &str = "fbb5a6f5ec8f79e0a4d39cc6d8654b5c";

pub fn api_key() -> &'static str {
    TMDB_API_KEY
}

// Providers wrap years in parentheses OR square brackets ("Round 6 [2021]").
static YEAR_IN_BRACKETS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[(\[](\d{4})[)\]]").expect("valid regex"));
static YEAR_TRAILING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(19\d{2}|20\d{2})\b").expect("valid regex"));
// Trailing release tags ("SEVEN SNIPERS (2026) LEG") sabotage TMDB searches.
static RELEASE_TAG_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[\s\-–]*\b(LEG|DUB|LEGENDADO|DUBLADO|NACIONAL|NAC|4K|FHD|UHD|HD|CAM|HDCAM|HDTS)\s*$")
        .expect("valid regex")
});

pub fn extract_year_from_title(title: &str) -> Option<i32> {
    if let Some(caps) = YEAR_IN_BRACKETS.captures(title) {
        return caps.get(1).and_then(|m| m.as_str().parse().ok());
    }
    YEAR_TRAILING
        .captures(title)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn clean_title_for_search(title: &str) -> String {
    let without_year = YEAR_IN_BRACKETS.replace_all(title, " ");
    let mut cleaned = without_year.trim().to_string();
    loop {
        let next = RELEASE_TAG_SUFFIX.replace(&cleaned, "").trim().to_string();
        if next == cleaned {
            break;
        }
        cleaned = next;
    }
    cleaned
}

fn is_mostly_uppercase(title: &str) -> bool {
    let letters: Vec<char> = title.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() < 3 {
        return false;
    }
    let upper = letters.iter().filter(|c| c.is_uppercase()).count();
    upper * 100 / letters.len() >= 80
}

fn to_title_case(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out = first.to_uppercase().to_string();
            out.push_str(&chars.as_str().to_lowercase());
            out
        }
    }
}

fn normalize_title_for_search(title: &str) -> String {
    let cleaned = clean_title_for_search(title);
    if is_mostly_uppercase(&cleaned) {
        cleaned
            .split_whitespace()
            .map(to_title_case)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        cleaned
    }
}

fn search_title_variants(title: &str) -> Vec<String> {
    let normalized = normalize_title_for_search(title);
    let mut variants = vec![normalized.clone()];

    for prefix in ["O ", "A ", "Os ", "As ", "Um ", "Uma ", "The ", "A "] {
        if let Some(stripped) = normalized.strip_prefix(prefix) {
            variants.push(stripped.trim().to_string());
        }
    }

    variants.retain(|v| !v.is_empty());
    variants.sort();
    variants.dedup();
    variants
}

fn is_blank(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|text| text.trim().is_empty())
}

fn tmdb_image_url(base: &str, path: Option<&str>) -> Option<String> {
    path.filter(|p| !p.is_empty())
        .map(|p| format!("{base}{p}"))
}

#[derive(serde::Deserialize)]
struct TmdbSearchResponse<T> {
    results: Vec<T>,
}

#[derive(serde::Deserialize)]
struct TmdbSearchHit {
    id: i64,
    title: Option<String>,
    original_title: Option<String>,
    name: Option<String>,
    original_name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    popularity: Option<f64>,
}

#[derive(serde::Deserialize)]
struct TmdbGenre {
    name: String,
}

#[derive(serde::Deserialize)]
struct TmdbCastMember {
    name: String,
    profile_path: Option<String>,
}

#[derive(serde::Deserialize)]
struct TmdbCredits {
    #[serde(default)]
    cast: Vec<TmdbCastMember>,
}

#[derive(serde::Deserialize)]
struct TmdbMovieDetail {
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f64>,
    #[serde(default)]
    genres: Vec<TmdbGenre>,
    credits: TmdbCredits,
}

#[derive(serde::Deserialize)]
struct TmdbTvDetail {
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f64>,
    #[serde(default)]
    genres: Vec<TmdbGenre>,
    credits: TmdbCredits,
}

struct TmdbEnrichment {
    overview: Option<String>,
    poster: Option<String>,
    backdrop: Option<String>,
    rating: Option<String>,
    genres: Option<String>,
    cast: Option<Vec<CastMember>>,
}

#[derive(Debug, Clone)]
pub struct DetailEnrichment {
    pub cast: Option<Vec<CastMember>>,
    pub tmdb_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SimilarCandidate {
    pub title: String,
    pub year: Option<i32>,
}

#[derive(serde::Deserialize)]
struct TmdbSimilarHit {
    title: Option<String>,
    name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
}

#[derive(serde::Deserialize)]
struct TmdbSimilarResponse {
    results: Vec<TmdbSimilarHit>,
}

fn format_vote_average(value: f64) -> String {
    format!("{:.1}", value)
}

fn format_genres(genres: &[TmdbGenre]) -> Option<String> {
    let names: Vec<String> = genres
        .iter()
        .map(|g| g.name.trim())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect();
    if names.is_empty() {
        None
    } else {
        Some(names.join(", "))
    }
}

fn parse_cast_members(credits: &TmdbCredits) -> Option<Vec<CastMember>> {
    let members: Vec<CastMember> = credits
        .cast
        .iter()
        .take(12)
        .filter_map(|member| {
            let name = member.name.trim();
            if name.is_empty() {
                return None;
            }
            Some(CastMember {
                name: name.to_string(),
                photo: tmdb_image_url(TMDB_PROFILE_BASE, member.profile_path.as_deref()),
            })
        })
        .collect();
    if members.is_empty() {
        None
    } else {
        Some(members)
    }
}

async fn tmdb_search_all(
    media_type: &str,
    api_key: &str,
    query: &str,
    year: Option<i32>,
) -> Vec<TmdbSearchHit> {
    let mut url = format!(
        "https://api.themoviedb.org/3/search/{media_type}?api_key={}&query={}&language=pt-BR&include_adult=false&region=BR",
        urlencoding::encode(api_key),
        urlencoding::encode(query),
    );
    if let Some(year) = year {
        if media_type == "movie" {
            url.push_str(&format!("&year={year}"));
        } else {
            url.push_str(&format!("&first_air_date_year={year}"));
        }
    }

    let client = reqwest::Client::new();
    let response = match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => response,
        _ => return Vec::new(),
    };

    response
        .json::<TmdbSearchResponse<TmdbSearchHit>>()
        .await
        .map(|body| body.results)
        .unwrap_or_default()
}

fn normalize_for_compare(value: &str) -> String {
    clean_title_for_search(value)
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_alphanumeric() || ch.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn title_similarity(query: &str, candidate: &str) -> f64 {
    let query_norm = normalize_for_compare(query);
    let candidate_norm = normalize_for_compare(candidate);
    if query_norm.is_empty() || candidate_norm.is_empty() {
        return 0.0;
    }
    if query_norm == candidate_norm {
        return 1.0;
    }
    if candidate_norm.contains(&query_norm) || query_norm.contains(&candidate_norm) {
        return 0.88;
    }

    let query_words: std::collections::HashSet<&str> = query_norm.split_whitespace().collect();
    let candidate_words: std::collections::HashSet<&str> =
        candidate_norm.split_whitespace().collect();
    if query_words.is_empty() || candidate_words.is_empty() {
        return 0.0;
    }

    let intersection = query_words.intersection(&candidate_words).count();
    let union = query_words.union(&candidate_words).count();
    intersection as f64 / union as f64
}

fn year_from_date(date: &Option<String>) -> Option<i32> {
    date.as_ref()
        .and_then(|value| value.get(0..4))
        .and_then(|value| value.parse().ok())
}

fn score_search_hit(
    hit: &TmdbSearchHit,
    query: &str,
    expected_year: Option<i32>,
) -> f64 {
    let titles = [
        hit.title.as_deref(),
        hit.original_title.as_deref(),
        hit.name.as_deref(),
        hit.original_name.as_deref(),
    ];
    let mut best_similarity = 0.0f64;
    for title in titles.into_iter().flatten() {
        best_similarity = best_similarity.max(title_similarity(query, title));
    }

    if best_similarity < 0.45 {
        return 0.0;
    }

    let mut score = best_similarity * 100.0;
    let hit_year = year_from_date(&hit.release_date).or_else(|| year_from_date(&hit.first_air_date));
    if let (Some(expected), Some(found)) = (expected_year, hit_year) {
        if expected == found {
            score += 30.0;
        } else if (expected - found).abs() == 1 {
            score += 12.0;
        } else if (expected - found).abs() > 2 {
            score -= 25.0;
        }
    }
    if let Some(popularity) = hit.popularity {
        score += popularity.min(40.0) * 0.15;
    }
    score
}

async fn ranked_tmdb_ids(media_type: &str, title: &str, api_key: &str) -> Vec<i64> {
    let expected_year = extract_year_from_title(title);
    let mut scored: Vec<(i64, f64)> = Vec::new();

    for query in search_title_variants(title) {
        let mut hits = tmdb_search_all(media_type, api_key, &query, expected_year).await;
        if hits.is_empty() && expected_year.is_some() {
            hits = tmdb_search_all(media_type, api_key, &query, None).await;
        }

        for hit in hits {
            let score = score_search_hit(&hit, &query, expected_year);
            if score > 0.0 {
                scored.push((hit.id, score));
            }
        }
    }

    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.dedup_by_key(|entry| entry.0);
    scored.into_iter().map(|entry| entry.0).collect()
}

async fn fetch_movie_detail(tmdb_id: i64, api_key: &str, language: &str) -> Option<TmdbMovieDetail> {
    let url = format!(
        "https://api.themoviedb.org/3/movie/{tmdb_id}?api_key={}&language={}&append_to_response=credits",
        urlencoding::encode(api_key),
        urlencoding::encode(language),
    );
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json().await.ok()
}

async fn fetch_tv_detail(tmdb_id: i64, api_key: &str, language: &str) -> Option<TmdbTvDetail> {
    let url = format!(
        "https://api.themoviedb.org/3/tv/{tmdb_id}?api_key={}&language={}&append_to_response=credits",
        urlencoding::encode(api_key),
        urlencoding::encode(language),
    );
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json().await.ok()
}

async fn load_movie_enrichment(tmdb_id: i64, api_key: &str) -> Option<TmdbEnrichment> {
    let detail_pt = fetch_movie_detail(tmdb_id, api_key, "pt-BR").await?;
    let overview_pt = detail_pt
        .overview
        .as_ref()
        .filter(|text| !text.trim().is_empty())
        .cloned();

    let overview = if overview_pt.is_some() {
        overview_pt
    } else {
        fetch_movie_detail(tmdb_id, api_key, "en-US")
            .await
            .and_then(|detail| {
                detail
                    .overview
                    .filter(|text| !text.trim().is_empty())
            })
    };

    Some(TmdbEnrichment {
        overview,
        poster: tmdb_image_url(TMDB_IMAGE_BASE, detail_pt.poster_path.as_deref()),
        backdrop: tmdb_image_url(TMDB_BACKDROP_BASE, detail_pt.backdrop_path.as_deref()),
        rating: detail_pt
            .vote_average
            .filter(|value| *value > 0.0)
            .map(format_vote_average),
        genres: format_genres(&detail_pt.genres),
        cast: parse_cast_members(&detail_pt.credits),
    })
}

async fn load_tv_enrichment(tmdb_id: i64, api_key: &str) -> Option<TmdbEnrichment> {
    let detail_pt = fetch_tv_detail(tmdb_id, api_key, "pt-BR").await?;
    let overview_pt = detail_pt
        .overview
        .as_ref()
        .filter(|text| !text.trim().is_empty())
        .cloned();

    let overview = if overview_pt.is_some() {
        overview_pt
    } else {
        fetch_tv_detail(tmdb_id, api_key, "en-US")
            .await
            .and_then(|detail| {
                detail
                    .overview
                    .filter(|text| !text.trim().is_empty())
            })
    };

    Some(TmdbEnrichment {
        overview,
        poster: tmdb_image_url(TMDB_IMAGE_BASE, detail_pt.poster_path.as_deref()),
        backdrop: tmdb_image_url(TMDB_BACKDROP_BASE, detail_pt.backdrop_path.as_deref()),
        rating: detail_pt
            .vote_average
            .filter(|value| *value > 0.0)
            .map(format_vote_average),
        genres: format_genres(&detail_pt.genres),
        cast: parse_cast_members(&detail_pt.credits),
    })
}

fn apply_enrichment_to_movie(movie: &mut Movie, enrichment: &TmdbEnrichment, trusted_match: bool) {
    let has_overview = enrichment
        .overview
        .as_ref()
        .is_some_and(|text| !text.trim().is_empty());

    if has_overview {
        movie.plot = enrichment.overview.clone();
    }

    if trusted_match || is_blank(&movie.poster) {
        if enrichment.poster.is_some() {
            movie.poster = enrichment.poster.clone();
        }
    }
    if trusted_match || is_blank(&movie.backdrop) {
        if enrichment.backdrop.is_some() {
            movie.backdrop = enrichment.backdrop.clone();
        }
    }
    if is_blank(&movie.rating) {
        movie.rating = enrichment.rating.clone();
    }
    if is_blank(&movie.genres) {
        movie.genres = enrichment.genres.clone();
    }
}

fn apply_enrichment_to_series(series: &mut Series, enrichment: &TmdbEnrichment, trusted_match: bool) {
    let has_overview = enrichment
        .overview
        .as_ref()
        .is_some_and(|text| !text.trim().is_empty());

    if has_overview {
        series.plot = enrichment.overview.clone();
    }

    if trusted_match || is_blank(&series.poster) {
        if enrichment.poster.is_some() {
            series.poster = enrichment.poster.clone();
        }
    }
    if trusted_match || is_blank(&series.backdrop) {
        if enrichment.backdrop.is_some() {
            series.backdrop = enrichment.backdrop.clone();
        }
    }
    if is_blank(&series.rating) {
        series.rating = enrichment.rating.clone();
    }
    if is_blank(&series.genres) {
        series.genres = enrichment.genres.clone();
    }
}

async fn resolve_movie_enrichment(title: &str, api_key: &str) -> Option<(i64, TmdbEnrichment)> {
    let ids = ranked_tmdb_ids("movie", title, api_key).await;
    let mut fallback: Option<(i64, TmdbEnrichment)> = None;

    for id in ids.into_iter().take(2) {
        let Some(enrichment) = load_movie_enrichment(id, api_key).await else {
            continue;
        };

        if enrichment
            .overview
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
        {
            return Some((id, enrichment));
        }

        if fallback.is_none()
            && (enrichment.poster.is_some() || enrichment.backdrop.is_some())
        {
            fallback = Some((id, enrichment));
        }
    }

    fallback
}

async fn resolve_tv_enrichment(title: &str, api_key: &str) -> Option<(i64, TmdbEnrichment)> {
    let ids = ranked_tmdb_ids("tv", title, api_key).await;
    let mut fallback: Option<(i64, TmdbEnrichment)> = None;

    for id in ids.into_iter().take(2) {
        let Some(enrichment) = load_tv_enrichment(id, api_key).await else {
            continue;
        };

        if enrichment
            .overview
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
        {
            return Some((id, enrichment));
        }

        if fallback.is_none()
            && (enrichment.poster.is_some() || enrichment.backdrop.is_some())
        {
            fallback = Some((id, enrichment));
        }
    }

    fallback
}

fn similar_hit_to_candidate(hit: &TmdbSimilarHit) -> Option<SimilarCandidate> {
    let title = hit
        .title
        .as_deref()
        .or(hit.name.as_deref())?
        .trim();
    if title.is_empty() {
        return None;
    }
    let year = year_from_date(&hit.release_date).or_else(|| year_from_date(&hit.first_air_date));
    Some(SimilarCandidate {
        title: title.to_string(),
        year,
    })
}

async fn fetch_similar_candidates(
    media_type: &str,
    tmdb_id: i64,
    api_key: &str,
) -> Vec<SimilarCandidate> {
    let url = format!(
        "https://api.themoviedb.org/3/{media_type}/{tmdb_id}/similar?api_key={}&language=pt-BR",
        urlencoding::encode(api_key),
    );
    let client = reqwest::Client::new();
    let response = match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => response,
        _ => return Vec::new(),
    };
    let body = match response.json::<TmdbSimilarResponse>().await {
        Ok(body) => body,
        Err(_) => return Vec::new(),
    };

    body.results
        .iter()
        .filter_map(similar_hit_to_candidate)
        .take(20)
        .collect()
}

pub async fn fetch_similar_movie_candidates(
    tmdb_id: i64,
    api_key: &str,
) -> Vec<SimilarCandidate> {
    fetch_similar_candidates("movie", tmdb_id, api_key).await
}

pub async fn fetch_similar_series_candidates(
    tmdb_id: i64,
    api_key: &str,
) -> Vec<SimilarCandidate> {
    fetch_similar_candidates("tv", tmdb_id, api_key).await
}

pub async fn enrich_movie(movie: &mut Movie, api_key: &str) -> AppResult<DetailEnrichment> {
    if search_title_variants(&movie.name).is_empty() {
        return Ok(DetailEnrichment {
            cast: None,
            tmdb_id: None,
        });
    }

    let Some((tmdb_id, enrichment)) = resolve_movie_enrichment(&movie.name, api_key).await else {
        return Ok(DetailEnrichment {
            cast: None,
            tmdb_id: None,
        });
    };

    let trusted_match = enrichment
        .overview
        .as_ref()
        .is_some_and(|text| !text.trim().is_empty());
    let cast = enrichment.cast.clone();
    apply_enrichment_to_movie(movie, &enrichment, trusted_match);
    Ok(DetailEnrichment {
        cast,
        tmdb_id: Some(tmdb_id),
    })
}

pub async fn enrich_series(series: &mut Series, api_key: &str) -> AppResult<DetailEnrichment> {
    if search_title_variants(&series.name).is_empty() {
        return Ok(DetailEnrichment {
            cast: None,
            tmdb_id: None,
        });
    }

    let Some((tmdb_id, enrichment)) = resolve_tv_enrichment(&series.name, api_key).await else {
        return Ok(DetailEnrichment {
            cast: None,
            tmdb_id: None,
        });
    };

    let trusted_match = enrichment
        .overview
        .as_ref()
        .is_some_and(|text| !text.trim().is_empty());
    let cast = enrichment.cast.clone();
    apply_enrichment_to_series(series, &enrichment, trusted_match);
    Ok(DetailEnrichment {
        cast,
        tmdb_id: Some(tmdb_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_year_from_title() {
        assert_eq!(extract_year_from_title("Inception (2010)"), Some(2010));
        assert_eq!(extract_year_from_title("Round 6 [2021]"), Some(2021));
        assert_eq!(extract_year_from_title("Breaking Bad 2008"), Some(2008));
        assert_eq!(extract_year_from_title("No Year"), None);
    }

    #[test]
    fn cleans_title_for_search() {
        assert_eq!(clean_title_for_search("Inception (2010)"), "Inception");
        assert_eq!(clean_title_for_search("Round 6 [2021]"), "Round 6");
        assert_eq!(
            clean_title_for_search("SEVEN SNIPERS (2026) LEG"),
            "SEVEN SNIPERS"
        );
        assert_eq!(
            clean_title_for_search("63 HORAS DE PANICO (2026) LEG DUB"),
            "63 HORAS DE PANICO"
        );
        // Tags only strip at the end — words inside titles stay intact.
        assert_eq!(
            clean_title_for_search("Legendado da Meia-Noite (2020)"),
            "Legendado da Meia-Noite"
        );
    }

    #[test]
    fn normalizes_uppercase_titles() {
        assert_eq!(
            normalize_title_for_search("O SEQUESTRO (2017)"),
            "O Sequestro"
        );
    }

    #[test]
    fn builds_search_variants_with_and_without_articles() {
        let variants = search_title_variants("O SEQUESTRO (2017)");
        assert!(variants.contains(&"O Sequestro".to_string()));
        assert!(variants.contains(&"Sequestro".to_string()));
    }

    #[test]
    fn treats_blank_strings_as_missing() {
        assert!(is_blank(&None));
        assert!(is_blank(&Some(String::new())));
        assert!(is_blank(&Some("   ".to_string())));
        assert!(!is_blank(&Some("Plot".to_string())));
    }
}
