use crate::error::{AppError, AppResult};

use super::types::{
    XtreamCategory, XtreamLiveStream, XtreamSeries, XtreamSeriesInfo, XtreamVodStream,
};

pub struct XtreamClient {
    base_url: String,
    username: String,
    password: String,
    http: reqwest::Client,
}

impl XtreamClient {
    pub fn new(base_url: String, username: String, password: String) -> Self {
        Self {
            base_url,
            username,
            password,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }

    fn api_url(&self, action: &str) -> String {
        format!(
            "{}/player_api.php?username={}&password={}&action={}",
            self.base_url.trim_end_matches('/'),
            urlencoding::encode(&self.username),
            urlencoding::encode(&self.password),
            action
        )
    }

    fn series_info_url(&self, series_id: i64) -> String {
        format!("{}&series_id={}", self.api_url("get_series_info"), series_id)
    }

    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> AppResult<T> {
        let response = self.http.get(url).send().await?;
        if !response.status().is_success() {
            return Err(AppError::msg(format!(
                "Xtream API error: HTTP {}",
                response.status()
            )));
        }
        let body = response.text().await?;
        serde_json::from_str(&body).map_err(|e| {
            AppError::msg(format!("Failed to parse Xtream API response: {e}"))
        })
    }

    pub async fn get_vod_categories(&self) -> AppResult<Vec<XtreamCategory>> {
        self.fetch_json(&self.api_url("get_vod_categories")).await
    }

    pub async fn get_vod_streams(&self) -> AppResult<Vec<XtreamVodStream>> {
        self.fetch_json(&self.api_url("get_vod_streams")).await
    }

    pub async fn get_live_categories(&self) -> AppResult<Vec<XtreamCategory>> {
        self.fetch_json(&self.api_url("get_live_categories")).await
    }

    pub async fn get_series_categories(&self) -> AppResult<Vec<XtreamCategory>> {
        self.fetch_json(&self.api_url("get_series_categories")).await
    }

    pub async fn get_series(&self) -> AppResult<Vec<XtreamSeries>> {
        self.fetch_json(&self.api_url("get_series")).await
    }

    pub async fn get_series_info(&self, series_id: i64) -> AppResult<XtreamSeriesInfo> {
        self.fetch_json(&self.series_info_url(series_id)).await
    }

    pub async fn get_live_streams(&self) -> AppResult<Vec<XtreamLiveStream>> {
        self.fetch_json(&self.api_url("get_live_streams")).await
    }

    pub fn short_epg_url(&self, stream_id: i64, limit: i64) -> String {
        format!(
            "{}&stream_id={}&limit={}",
            self.api_url("get_short_epg"),
            stream_id,
            limit.max(1)
        )
    }

    pub async fn get_short_epg(
        &self,
        stream_id: i64,
        limit: i64,
    ) -> AppResult<super::types::XtreamShortEpgResponse> {
        self.fetch_json(&self.short_epg_url(stream_id, limit)).await
    }

    pub fn build_live_url(&self, stream_id: i64, extension: &str) -> String {
        format!(
            "{}/live/{}/{}/{}.{}",
            self.base_url.trim_end_matches('/'),
            self.username,
            self.password,
            stream_id,
            extension
        )
    }

    pub fn build_vod_url(&self, stream_id: i64, extension: &str) -> String {
        format!(
            "{}/movie/{}/{}/{}.{}",
            self.base_url.trim_end_matches('/'),
            self.username,
            self.password,
            stream_id,
            extension
        )
    }

    pub fn build_series_episode_url(&self, stream_id: i64, extension: &str) -> String {
        format!(
            "{}/series/{}/{}/{}.{}",
            self.base_url.trim_end_matches('/'),
            self.username,
            self.password,
            stream_id,
            extension
        )
    }
}
