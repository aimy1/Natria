//! CosyVoice 本地多语言零样本声音克隆 TTS 引擎客户端。
//!
//! 对接本地 CosyVoice API 服务（默认端口 9233），
//! 传入参考音频（Prompt Audio）与参考文本（Prompt Text），实现高保真自然多情感声音克隆。

use crate::voice::types::VoiceConfig;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::json;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct CosyVoiceEngine {
    client: Client,
}

impl CosyVoiceEngine {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
        }
    }

    /// 解析参考音频路径
    fn resolve_ref_audio_path(&self, raw: Option<&str>) -> Option<PathBuf> {
        let clean = raw?.trim().trim_start_matches("local:");
        if clean.is_empty() {
            return None;
        }

        let p = Path::new(clean);
        if p.is_absolute() && p.exists() {
            return Some(p.to_path_buf());
        }

        let voices_cand = PathBuf::from("voices").join(clean);
        if voices_cand.exists() {
            if let Ok(abs) = std::fs::canonicalize(&voices_cand) {
                return Some(abs);
            }
            return Some(voices_cand);
        }

        None
    }

    pub async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Vec<u8>> {
        let endpoint_raw = config
            .endpoint
            .as_deref()
            .unwrap_or("http://127.0.0.1:9233");
        let base_url = endpoint_raw.trim_end_matches('/');

        let prompt_text = config.prompt_text.as_deref().unwrap_or("");
        let prompt_audio_path = self.resolve_ref_audio_path(
            config
                .prompt_audio
                .as_deref()
                .or_else(|| Some(config.voice.as_str()).filter(|v| v.starts_with("local:"))),
        );

        // CosyVoice 标准 zero-shot 接口或通用 /tts
        let url = if base_url.ends_with("/inference_zero_shot") || base_url.ends_with("/tts") {
            base_url.to_string()
        } else {
            format!("{base_url}/inference_zero_shot")
        };

        // 尝试构建 JSON 或 Multipart
        let body = json!({
            "tts_text": text,
            "text": text,
            "prompt_text": prompt_text,
            "prompt_wav": prompt_audio_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            "ref_audio_path": prompt_audio_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
        });

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = &config.api_key {
            req = req.bearer_auth(key);
        }

        let resp_res = req.send().await;
        let resp = match resp_res {
            Ok(r) => r,
            Err(e) => {
                // 降级尝试向 /tts 发生请求
                let fallback_url = format!("{base_url}/tts");
                let mut fallback_req = self.client.post(&fallback_url).json(&body);
                if let Some(key) = &config.api_key {
                    fallback_req = fallback_req.bearer_auth(key);
                }
                fallback_req
                    .send()
                    .await
                    .with_context(|| format!("Failed to send request to CosyVoice API ({base_url}): {e}"))?
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            bail!("CosyVoice synthesis failed ({status}): {err_body}");
        }

        let bytes = resp.bytes().await?;
        if bytes.is_empty() {
            bail!("CosyVoice returned empty audio data");
        }
        Ok(bytes.to_vec())
    }
}
