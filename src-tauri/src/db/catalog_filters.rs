use crate::db::models::CatalogItem;
use std::collections::HashSet;

use crate::services::content_kind::{
    has_movie_year, is_channel_like_name, title_without_quality_suffix,
};

const MIN_VOD_MOVIE_TITLE_LEN: usize = 8;

/// Rolling window for the "Lançamentos" sort (30 days).
pub const RELEASES_WINDOW_SECS: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSort {
    Az,
    Recent,
    Releases,
}

impl CatalogSort {
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("az") => Self::Az,
            Some("releases") => Self::Releases,
            _ => Self::Recent,
        }
    }

    pub fn movies_order_by(self) -> &'static str {
        match self {
            Self::Az => "name ASC",
            Self::Recent => "added_at DESC, sort_order ASC, name ASC",
            Self::Releases => "added_at DESC",
        }
    }

    pub fn series_order_by(self) -> &'static str {
        match self {
            Self::Az => "name ASC",
            Self::Recent => "added_at DESC, sort_order ASC, name ASC",
            Self::Releases => "effective_added_at DESC",
        }
    }
}

pub fn releases_since_ts(now: i64) -> i64 {
    now.saturating_sub(RELEASES_WINDOW_SECS)
}

pub fn normalize_catalog_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_name(name: &str) -> String {
    normalize_catalog_name(name)
}

/// True when a row in `movies` is real on-demand content (not a live channel misclassified as VOD).
pub fn is_valid_vod_movie(name: &str) -> bool {
    if is_channel_like_name(name) {
        return false;
    }
    if has_movie_year(name) {
        return true;
    }
    title_without_quality_suffix(name).len() >= MIN_VOD_MOVIE_TITLE_LEN
}

/// True when a row in `series` is real on-demand content (not a live channel misclassified as VOD).
pub fn is_valid_vod_series(name: &str, category: Option<&str>) -> bool {
    !is_junk_series_name(name, category)
}

pub fn is_junk_series_name(name: &str, category: Option<&str>) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown series") {
        return true;
    }
    if is_channel_like_name(trimmed) {
        return true;
    }
    // Series titles legitimately include release years (e.g. "Vikings (2013)").
    if let Some(cat) = category.filter(|c| !c.trim().is_empty()) {
        if normalize_catalog_name(trimmed) == normalize_catalog_name(cat) {
            return true;
        }
    }
    false
}

pub fn filter_catalog_items(mut items: Vec<CatalogItem>, limit: usize) -> Vec<CatalogItem> {
    let mut seen_names = HashSet::new();
    let mut seen_posters = HashSet::new();
    let mut result = Vec::with_capacity(limit.min(items.len()));

    for item in items.drain(..) {
        if result.len() >= limit {
            break;
        }

        if is_channel_like_name(&item.name) {
            continue;
        }

        if item.item_type == "movie" && !is_valid_vod_movie(&item.name) {
            continue;
        }

        let poster = item
            .poster
            .as_deref()
            .or(item.backdrop.as_deref())
            .map(str::trim)
            .filter(|p| !p.is_empty());
        if poster.is_none() {
            continue;
        }

        let norm_name = normalize_name(&item.name);
        if !norm_name.is_empty() && !seen_names.insert(norm_name) {
            continue;
        }

        if let Some(p) = poster {
            let poster_key = p.to_lowercase();
            if !seen_posters.insert(poster_key) {
                continue;
            }
        }

        result.push(item);
    }

    result
}

pub fn filter_recent_series_items(mut items: Vec<CatalogItem>, limit: usize) -> Vec<CatalogItem> {
    let mut seen_names = HashSet::new();
    let mut seen_ids = HashSet::new();
    let mut result = Vec::with_capacity(limit.min(items.len()));

    for item in items.drain(..) {
        if result.len() >= limit {
            break;
        }

        if item.item_type != "series" {
            continue;
        }

        if is_junk_series_name(&item.name, None) {
            continue;
        }

        let poster = item
            .poster
            .as_deref()
            .or(item.backdrop.as_deref())
            .map(str::trim)
            .filter(|p| !p.is_empty());
        if poster.is_none() {
            continue;
        }

        if !seen_ids.insert(item.id.clone()) {
            continue;
        }

        let norm_name = normalize_catalog_name(&item.name);
        if !norm_name.is_empty() && !seen_names.insert(norm_name) {
            continue;
        }

        result.push(item);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, name: &str, poster: Option<&str>) -> CatalogItem {
        CatalogItem {
            id: id.to_string(),
            item_type: "movie".to_string(),
            name: name.to_string(),
            poster: poster.map(str::to_string),
            backdrop: None,
            added_at: 100,
        }
    }

    #[test]
    fn filters_duplicates_and_channel_entries() {
        let items = vec![
            item("1", "TC PREMIUM", Some("http://logo.com/tc.png")),
            item("2", "Inception", Some("http://logo.com/inception.png")),
            item("3", "INCEPTION", Some("http://logo.com/inception.png")),
            item("4", "Another Film", Some("http://logo.com/inception.png")),
            item("5", "No Poster Movie", None),
            item("6", "MEGAPIX FHD", Some("http://logo.com/megapix.png")),
        ];

        let filtered = filter_catalog_items(items, 10);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "2");
    }

    #[test]
    fn is_valid_vod_movie_rejects_channels_but_keeps_real_titles() {
        assert!(!is_valid_vod_movie("TC ACTION"));
        assert!(!is_valid_vod_movie("TELECINE PIPOCA FHD"));
        assert!(!is_valid_vod_movie("USA HD"));
        assert!(!is_valid_vod_movie("MEGAPIX FHD"));
        assert!(!is_valid_vod_movie("MEGAPIX FHD*"));
        assert!(!is_valid_vod_movie("MEGAPIX FHDᴮᴿ"));
        assert!(!is_valid_vod_movie("MEGAPIX HD*"));
        assert!(!is_valid_vod_movie("MEGAPIX BR"));
        assert!(!is_valid_vod_movie("MEGAPIX SD*"));
        assert!(!is_valid_vod_movie("USA FHD*"));
        assert!(!is_valid_vod_movie("USA HDᴮᴿ"));
        assert!(!is_valid_vod_movie("UNIVERSAL PREMIER"));
        assert!(!is_valid_vod_movie("HD"));
        assert!(!is_valid_vod_movie("Old"));
        assert!(is_valid_vod_movie("Inception (2010)"));
        assert!(is_valid_vod_movie("Dune: Part Two"));
        assert!(is_valid_vod_movie("Real Movie"));
    }

    #[test]
    fn filter_recent_series_dedupes_by_name_and_requires_series_type() {
        let items = vec![
            CatalogItem {
                id: "s1".to_string(),
                item_type: "series".to_string(),
                name: "Pacificador".to_string(),
                poster: Some("http://example.com/p1.png".to_string()),
                backdrop: None,
                added_at: 300,
            },
            CatalogItem {
                id: "s2".to_string(),
                item_type: "series".to_string(),
                name: "PACIFICADOR".to_string(),
                poster: Some("http://example.com/p2.png".to_string()),
                backdrop: None,
                added_at: 200,
            },
            CatalogItem {
                id: "m1".to_string(),
                item_type: "movie".to_string(),
                name: "Some Movie".to_string(),
                poster: Some("http://example.com/m.png".to_string()),
                backdrop: None,
                added_at: 400,
            },
            CatalogItem {
                id: "s3".to_string(),
                item_type: "series".to_string(),
                name: "TC PREMIUM".to_string(),
                poster: Some("http://example.com/tc.png".to_string()),
                backdrop: None,
                added_at: 500,
            },
        ];

        let filtered = filter_recent_series_items(items, 20);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "s1");
    }
}
