pub mod client;
pub mod types;

pub use client::XtreamClient;
pub use types::{
    XtreamCategory, XtreamEpisode, XtreamEpgListing, XtreamLiveStream, XtreamSeason,
    XtreamSeries, XtreamSeriesDetail, XtreamSeriesInfo, XtreamShortEpgResponse,
    XtreamVodStream, decode_xtream_epg_text,
};

#[cfg(test)]
mod tests {
    use super::types::*;
    use super::{decode_xtream_epg_text, XtreamClient};
    use std::collections::HashMap;

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("parsers")
            .join("xtream")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn deserializes_category_fixtures() {
        let vod_json = std::fs::read_to_string(fixture_path("vod_categories.json")).unwrap();
        let vod_cats: Vec<XtreamCategory> = serde_json::from_str(&vod_json).unwrap();
        assert_eq!(vod_cats[0].category_name, "Filmes");

        let live_json = std::fs::read_to_string(fixture_path("live_categories.json")).unwrap();
        let live_cats: Vec<XtreamCategory> = serde_json::from_str(&live_json).unwrap();
        assert_eq!(live_cats[0].category_name, "Canais");

        let series_json = std::fs::read_to_string(fixture_path("series_categories.json")).unwrap();
        let series_cats: Vec<XtreamCategory> = serde_json::from_str(&series_json).unwrap();
        assert_eq!(series_cats[0].category_name, "Séries");
    }

    #[test]
    fn deserializes_vod_streams_fixture() {
        let json = std::fs::read_to_string(fixture_path("vod_streams.json")).unwrap();
        let streams: Vec<XtreamVodStream> = serde_json::from_str(&json).unwrap();
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0].stream_id, 1001);
        assert_eq!(streams[0].name, "Sample Movie");
        assert_eq!(streams[0].container_extension.as_deref(), Some("mp4"));
        assert_eq!(streams[1].stream_id, 1002);
        assert_eq!(streams[1].rating.as_deref(), Some("7.5"));
    }

    #[test]
    fn deserializes_series_fixture() {
        let json = std::fs::read_to_string(fixture_path("series.json")).unwrap();
        let series: Vec<XtreamSeries> = serde_json::from_str(&json).unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].series_id, 2001);
        assert_eq!(series[0].name, "Sample Series");
        assert_eq!(series[0].genre.as_deref(), Some("Drama"));
        assert_eq!(series[1].plot.as_deref(), Some("A second show."));
    }

    #[test]
    fn deserializes_live_streams_fixture() {
        let json = std::fs::read_to_string(fixture_path("live_streams.json")).unwrap();
        let streams: Vec<XtreamLiveStream> = serde_json::from_str(&json).unwrap();
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0].stream_id, 3001);
        assert_eq!(streams[0].name, "News Channel");
        assert_eq!(streams[0].epg_channel_id.as_deref(), Some("news.hd"));
        assert_eq!(streams[1].category_id.as_deref(), Some("10"));
    }

    #[test]
    fn deserializes_series_info_fixture() {
        let json = std::fs::read_to_string(fixture_path("series_info.json")).unwrap();
        let info: XtreamSeriesInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.info.as_ref().unwrap().name.as_deref(), Some("Sample Series"));
        assert_eq!(info.seasons.as_ref().unwrap().len(), 2);

        let season1 = info.episodes.get("1").unwrap();
        assert_eq!(season1.len(), 2);
        assert_eq!(season1[0].id, 5001);
        assert_eq!(season1[0].episode_num, Some(1));
        assert_eq!(season1[0].title.as_deref(), Some("Pilot"));
        assert_eq!(season1[0].container_extension.as_deref(), Some("mp4"));

        let season2 = info.episodes.get("2").unwrap();
        assert_eq!(season2.len(), 1);
        assert_eq!(season2[0].id, 5003);
    }

    #[test]
    fn deserializes_episode_id_as_string() {
        let json = r#"{"id":"9999","episode_num":1,"title":"Test"}"#;
        let episode: XtreamEpisode = serde_json::from_str(json).unwrap();
        assert_eq!(episode.id, 9999);
    }

    #[test]
    fn builds_vod_url() {
        let client = XtreamClient::new(
            "http://example.com/".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        assert_eq!(
            client.build_vod_url(42, "mkv"),
            "http://example.com/movie/user/pass/42.mkv"
        );
    }

    #[test]
    fn builds_series_episode_url() {
        let client = XtreamClient::new(
            "http://example.com".to_string(),
            "user@name".to_string(),
            "p@ss".to_string(),
        );
        assert_eq!(
            client.build_series_episode_url(99, "mp4"),
            "http://example.com/series/user@name/p@ss/99.mp4"
        );
    }

    #[test]
    fn series_info_episodes_map_is_empty_by_default() {
        let json = r#"{"seasons":[],"info":{"name":"Empty"},"episodes":{}}"#;
        let info: XtreamSeriesInfo = serde_json::from_str(json).unwrap();
        assert!(info.episodes.is_empty());
        assert_eq!(info.episodes, HashMap::new());
    }

    #[test]
    fn deserializes_short_epg_fixture() {
        let json = std::fs::read_to_string(fixture_path("short_epg.json")).unwrap();
        let response: XtreamShortEpgResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.epg_listings.len(), 2);
        assert_eq!(
            decode_xtream_epg_text(response.epg_listings[0].title.as_deref().unwrap()),
            "Jornal Nacional"
        );
    }

    #[test]
    fn builds_short_epg_url() {
        let client = XtreamClient::new(
            "http://example.com/".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        let url = client.short_epg_url(3001, 6);
        assert!(url.contains("get_short_epg"));
        assert!(url.contains("stream_id=3001"));
        assert!(url.contains("limit=6"));
    }
}
