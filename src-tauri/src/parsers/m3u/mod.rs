pub mod classifier;
pub mod series_parser;

use crate::db::ids::stable_channel_id;
use crate::db::models::Channel;
use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::path::Path;
#[derive(Debug, Default)]
struct PendingChannel {
    name: Option<String>,
    logo: Option<String>,
    group_name: Option<String>,
    tvg_id: Option<String>,
}

pub struct M3uParser {
    profile_id: String,
    sort_order: i32,
}

impl M3uParser {
    pub fn new(profile_id: String) -> Self {
        Self {
            profile_id,
            sort_order: 0,
        }
    }

    pub async fn parse_url(&mut self, url: &str) -> AppResult<Vec<Channel>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(AppError::msg(format!(
                "Falha ao baixar lista: HTTP {}",
                response.status()
            )));
        }
        let mut channels = Vec::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut line_start = 0usize;
        let mut pending = PendingChannel::default();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(rel_pos) = buffer[line_start..].find('\n') {
                let end = line_start + rel_pos;
                let line = buffer[line_start..end].trim_end_matches('\r');
                self.process_line(line, &mut pending, &mut channels)?;
                line_start = end + 1;
            }
            if line_start > 0 {
                buffer.drain(..line_start);
                line_start = 0;
            }
        }
        if !buffer.trim().is_empty() {
            self.process_line(buffer.trim(), &mut pending, &mut channels)?;
        }
        if channels.is_empty() {
            return Err(AppError::msg("Lista M3U vazia ou inválida."));
        }
        Ok(channels)
    }

    pub async fn parse_file(&mut self, path: &str) -> AppResult<Vec<Channel>> {
        let content = tokio::fs::read_to_string(path).await?;
        self.parse_content(&content)
    }

    pub fn parse_content(&mut self, content: &str) -> AppResult<Vec<Channel>> {
        let mut channels = Vec::new();
        let mut pending = PendingChannel::default();
        for line in content.lines() {
            self.process_line(line, &mut pending, &mut channels)?;
        }
        if channels.is_empty() {
            return Err(AppError::msg("Lista M3U vazia ou inválida."));
        }
        Ok(channels)
    }

    fn process_line(
        &mut self,
        line: &str,
        pending: &mut PendingChannel,
        channels: &mut Vec<Channel>,
    ) -> AppResult<()> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        if line.starts_with("#EXTINF:") {
            let attrs = parse_extinf_attrs(line);
            pending.tvg_id = attrs.get("tvg-id").cloned();
            pending.logo = attrs.get("tvg-logo").cloned();
            pending.group_name = attrs
                .get("group-title")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            pending.name = extract_display_name(line);
            return Ok(());
        }

        if line.starts_with('#') {
            return Ok(());
        }

        if pending.name.is_some() {
            let name = pending.name.take().unwrap_or_else(|| "Canal".to_string());
            let channel = Channel {
                id: stable_channel_id(&self.profile_id, line),
                profile_id: self.profile_id.clone(),
                name,
                logo: pending.logo.take(),
                group_name: pending.group_name.take(),
                stream_url: line.to_string(),
                tvg_id: pending.tvg_id.take(),
                sort_order: self.sort_order,
            };
            self.sort_order += 1;
            channels.push(channel);
        }

        Ok(())
    }
}

fn parse_extinf_attrs(line: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let parts: Vec<&str> = line.split('"').collect();
    for i in (1..parts.len()).step_by(2) {
        let value = parts[i];
        if let Some(before) = parts.get(i.wrapping_sub(1)) {
            if let Some(eq_pos) = before.rfind('=') {
                let key_part = before[..eq_pos].trim();
                let key = key_part
                    .split_whitespace()
                    .last()
                    .unwrap_or(key_part);
                if !key.is_empty() {
                    attrs.insert(key.to_string(), value.to_string());
                }
            }
        }
    }
    attrs
}

fn extract_display_name(line: &str) -> Option<String> {
    let comma = line.rfind(',')?;
    let name = line[comma + 1..].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub fn validate_source(url: Option<&str>, file_path: Option<&str>) -> AppResult<()> {
    match (url, file_path) {
        (Some(u), None) if !u.trim().is_empty() => {
            if !u.starts_with("http://") && !u.starts_with("https://") {
                return Err(AppError::msg("URL deve começar com http:// ou https://"));
            }
            Ok(())
        }
        (None, Some(p)) if !p.trim().is_empty() => {
            if !Path::new(p).exists() {
                return Err(AppError::msg("Arquivo M3U não encontrado."));
            }
            Ok(())
        }
        _ => Err(AppError::msg("Informe uma URL ou arquivo M3U.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_m3u() {
        let content = r#"#EXTM3U
#EXTINF:-1 tvg-id="1" tvg-logo="http://logo" group-title="News",Canal Um
http://example.com/1.m3u8
#EXTINF:-1 group-title="Sports",Canal Dois
http://example.com/2.ts
"#;
        let mut parser = M3uParser::new("profile-1".to_string());
        let channels = parser.parse_content(content).unwrap();
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].name, "Canal Um");
        assert_eq!(channels[0].group_name.as_deref(), Some("News"));
        assert_eq!(channels[1].stream_url, "http://example.com/2.ts");
    }

    #[test]
    fn parses_10k_fixture() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("channels-10000.m3u");
        if !fixture.exists() {
            eprintln!("Skipping 10k fixture test — file not found at {}", fixture.display());
            return;
        }
        let mut parser = M3uParser::new("bench".to_string());
        let channels = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { parser.parse_file(fixture.to_str().unwrap()).await })
        })
        .join()
        .unwrap()
        .unwrap();
        assert_eq!(channels.len(), 10000);
        assert_eq!(channels[0].sort_order, 0);
        assert_eq!(channels[9999].sort_order, 9999);
    }
}
