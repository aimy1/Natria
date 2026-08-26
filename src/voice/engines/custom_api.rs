//! 自定义 HTTP TTS 引擎客户端（支持 GPT-SoVITS / CosyVoice 等本地开源模型接口）。

use crate::voice::types::VoiceConfig;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::json;

#[derive(Clone)]
pub struct CustomHttpTtsEngine {
    client: Client,
}

impl CustomHttpTtsEngine {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
        }
    }

    pub async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Vec<u8>> {
        let Some(endpoint) = &config.endpoint else {
            bail!("Custom HTTP TTS requires 'endpoint' to be configured");
        };

        let body = json!({
            "text": text,
            "text_language": "auto",
            "character": config.voice,
        });

        let mut req = self.client.post(endpoint).json(&body);
        if let Some(key) = &config.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .context("Failed to send Custom HTTP TTS request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            bail!("Custom TTS failed with status {status}: {err_body}");
        }

        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}
