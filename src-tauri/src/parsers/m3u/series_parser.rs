use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEpisode {
    pub series_name: String,
    pub season: i32,
    pub episode: i32,
    pub episode_title: String,
}

static SXXEXX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(.+?)\s+S(\d+)\s*E(\d+)(?:\s*[-–—]\s*(.+))?\s*$").unwrap()
});

static NXNN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(.+?)\s+(\d+)x(\d+)(?:\s*[-–—]\s*(.+))?\s*$").unwrap()
});

pub fn parse_series_title(name: &str) -> Option<ParsedEpisode> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(caps) = SXXEXX_RE.captures(trimmed) {
        return Some(ParsedEpisode {
            series_name: caps.get(1)?.as_str().trim().to_string(),
            season: caps.get(2)?.as_str().parse().ok()?,
            episode: caps.get(3)?.as_str().parse().ok()?,
            episode_title: caps
                .get(4)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default(),
        });
    }

    if let Some(caps) = NXNN_RE.captures(trimmed) {
        return Some(ParsedEpisode {
            series_name: caps.get(1)?.as_str().trim().to_string(),
            season: caps.get(2)?.as_str().parse().ok()?,
            episode: caps.get(3)?.as_str().parse().ok()?,
            episode_title: caps
                .get(4)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sxxexx_format() {
        let parsed = parse_series_title("Breaking Bad S01E05").unwrap();
        assert_eq!(parsed.series_name, "Breaking Bad");
        assert_eq!(parsed.season, 1);
        assert_eq!(parsed.episode, 5);
        assert_eq!(parsed.episode_title, "");
    }

    #[test]
    fn parses_sxx_exx_with_space() {
        let parsed = parse_series_title("Breaking Bad S01 E05").unwrap();
        assert_eq!(parsed.series_name, "Breaking Bad");
        assert_eq!(parsed.season, 1);
        assert_eq!(parsed.episode, 5);
        assert_eq!(parsed.episode_title, "");
    }

    #[test]
    fn parses_nxnn_format() {
        let parsed = parse_series_title("Breaking Bad 1x05").unwrap();
        assert_eq!(parsed.series_name, "Breaking Bad");
        assert_eq!(parsed.season, 1);
        assert_eq!(parsed.episode, 5);
        assert_eq!(parsed.episode_title, "");
    }

    #[test]
    fn parses_nxnn_with_episode_title() {
        let parsed = parse_series_title("Breaking Bad 1x05 - Pilot").unwrap();
        assert_eq!(parsed.series_name, "Breaking Bad");
        assert_eq!(parsed.season, 1);
        assert_eq!(parsed.episode, 5);
        assert_eq!(parsed.episode_title, "Pilot");
    }

    #[test]
    fn returns_none_for_non_series_titles() {
        assert!(parse_series_title("Breaking Bad").is_none());
        assert!(parse_series_title("Live News Channel").is_none());
        assert!(parse_series_title("").is_none());
    }
}
