//! OpenAI 兼容 TTS 引擎客户端。
//!
//! 支持标准 `/v1/audio/speech` 接口，可用于接入 OpenAI 官方 TTS 或兼容该规范的本地/自建服务。

use crate::voice::types::VoiceConfig;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::json;

#[derive(Clone)]
pub struct OpenAiTtsEngine {
    client: Client,
}

impl OpenAiTtsEngine {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
        }
    }

    pub async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Vec<u8>> {
        let endpoint = config
            .endpoint
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/audio/speech");

        let mut req = self.client.post(endpoint);

        if let Some(key) = &config.api_key {
            req = req.bearer_auth(key);
        }

        let speed = parse_rate_to_speed(&config.rate);

        let body = json!({
            "model": "tts-1",
            "input": text,
            "voice": config.voice,
            "response_format": "mp3",
            "speed": speed
        });

        let resp = req
            .json(&body)
            .send()
            .await
            .context("Failed to send OpenAI TTS request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            bail!("OpenAI TTS failed with status {status}: {err_body}");
        }

        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}

fn parse_rate_to_speed(rate_str: &str) -> f32 {
    let trimmed = rate_str.trim().trim_end_matches('%');
    if let Ok(val) = trimmed.parse::<f32>() {
        1.0 + (val / 100.0)
    } else {
        1.0
    }
}
