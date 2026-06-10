use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamCategory {
    pub category_id: String,
    pub category_name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamVodStream {
    pub stream_id: i64,
    pub name: String,
    pub stream_icon: Option<String>,
    pub cover: Option<String>,
    pub cover_big: Option<String>,
    pub movie_image: Option<String>,
    pub backdrop_path: Option<String>,
    pub rating: Option<String>,
    pub category_id: Option<String>,
    pub added: Option<String>,
    pub last_modified: Option<String>,
    pub container_extension: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamSeries {
    pub series_id: i64,
    pub name: String,
    pub cover: Option<String>,
    pub cover_big: Option<String>,
    pub backdrop_path: Option<String>,
    pub plot: Option<String>,
    pub genre: Option<String>,
    pub rating: Option<String>,
    pub category_id: Option<String>,
    pub added: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamLiveStream {
    pub stream_id: i64,
    pub name: String,
    pub stream_icon: Option<String>,
    pub category_id: Option<String>,
    pub epg_channel_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamSeason {
    pub season_number: Option<i32>,
    pub name: Option<String>,
    pub episode_count: Option<i32>,
    pub overview: Option<String>,
    pub cover: Option<String>,
    pub air_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamEpisode {
    #[serde(deserialize_with = "deserialize_flexible_i64")]
    pub id: i64,
    pub episode_num: Option<i32>,
    pub title: Option<String>,
    pub container_extension: Option<String>,
    pub season: Option<i32>,
    pub added: Option<String>,
    pub stream_icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamSeriesDetail {
    pub name: Option<String>,
    pub cover: Option<String>,
    pub cover_big: Option<String>,
    pub backdrop_path: Option<String>,
    pub plot: Option<String>,
    pub genre: Option<String>,
    pub rating: Option<String>,
    pub category_id: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct XtreamSeriesInfo {
    pub seasons: Option<Vec<XtreamSeason>>,
    pub info: Option<XtreamSeriesDetail>,
    pub episodes: HashMap<String, Vec<XtreamEpisode>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamShortEpgResponse {
    #[serde(default)]
    pub epg_listings: Vec<XtreamEpgListing>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct XtreamEpgListing {
    pub id: Option<String>,
    pub epg_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub channel_id: Option<String>,
    #[serde(deserialize_with = "deserialize_flexible_i64_option")]
    pub start_timestamp: Option<i64>,
    #[serde(deserialize_with = "deserialize_flexible_i64_option")]
    pub stop_timestamp: Option<i64>,
}

fn deserialize_flexible_i64_option<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct FlexibleI64OptionVisitor;

    impl<'de> Visitor<'de> for FlexibleI64OptionVisitor {
        type Value = Option<i64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null, an integer, or a string representing an integer")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(i64::try_from(value).map_err(de::Error::custom)?))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value.trim().is_empty() {
                return Ok(None);
            }
            value.parse().map(Some).map_err(de::Error::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value.trim().is_empty() {
                return Ok(None);
            }
            value.parse().map(Some).map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(FlexibleI64OptionVisitor)
}

pub fn decode_xtream_epg_text(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    use base64::Engine;
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
        if let Ok(decoded) = String::from_utf8(bytes) {
            if !decoded.is_empty() {
                return decoded;
            }
        }
    }
    trimmed.to_string()
}

fn deserialize_flexible_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct FlexibleI64Visitor;

    impl<'de> Visitor<'de> for FlexibleI64Visitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer or string representing an integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            i64::try_from(value).map_err(de::Error::custom)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(de::Error::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(FlexibleI64Visitor)
}
