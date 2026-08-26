//! GPT-SoVITS 本地零样本声音克隆 TTS 引擎客户端。
//!
//! 对接本地 GPT-SoVITS API 服务（默认端口 9880），
//! 传入参考音频（Prompt Audio）与参考文本（Prompt Text），实现任意文本的声音克隆朗读。

use crate::voice::types::VoiceConfig;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::json;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct GptSovitsEngine {
    client: Client,
}

impl GptSovitsEngine {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
        }
    }

    /// 解析参考音频的本地完整绝对路径，供 GPT-SoVITS 本地进程读取。
    fn resolve_ref_audio_path(&self, raw: Option<&str>) -> String {
        let Some(p) = raw else {
            return String::new();
        };
        let clean = p.trim().trim_start_matches("local:");
        if clean.is_empty() {
            return String::new();
        }

        let path = Path::new(clean);
        if path.is_absolute() && path.exists() {
            return clean.to_string();
        }

        // 尝试从 voices/ 目录查找
        let voices_cand = PathBuf::from("voices").join(clean);
        if voices_cand.exists() {
            if let Ok(abs) = std::fs::canonicalize(&voices_cand) {
                return abs.to_string_lossy().to_string();
            }
            return voices_cand.to_string_lossy().to_string();
        }

        clean.to_string()
    }

    pub async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Vec<u8>> {
        let endpoint_raw = config
            .endpoint
            .as_deref()
            .unwrap_or("http://127.0.0.1:9880");
        let base_url = endpoint_raw.trim_end_matches('/');

        // 自动探测 /tts 或 / 根路径
        let url = if base_url.ends_with("/tts") {
            base_url.to_string()
        } else {
            format!("{base_url}/tts")
        };

        let ref_audio = self.resolve_ref_audio_path(
            config
                .prompt_audio
                .as_deref()
                .or_else(|| Some(config.voice.as_str()).filter(|v| v.starts_with("local:"))),
        );

        let text_lang = config.text_lang.as_deref().unwrap_or("zh");
        let prompt_lang = config.prompt_lang.as_deref().unwrap_or("zh");
        let prompt_text = config.prompt_text.as_deref().unwrap_or("");

        let body = json!({
            "text": text,
            "text_lang": text_lang,
            "ref_audio_path": ref_audio,
            "prompt_text": prompt_text,
            "prompt_lang": prompt_lang,
            "media_type": "wav"
        });

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = &config.api_key {
            req = req.bearer_auth(key);
        }

        let resp_res = req.send().await;
        let resp = match resp_res {
            Ok(r) => r,
            Err(e) => {
                // 若 /tts 请求失败，尝试直接向根路径 POST
                let fallback_url = format!("{base_url}/");
                let mut fallback_req = self.client.post(&fallback_url).json(&body);
                if let Some(key) = &config.api_key {
                    fallback_req = fallback_req.bearer_auth(key);
                }
                fallback_req
                    .send()
                    .await
                    .with_context(|| format!("Failed to send request to GPT-SoVITS API ({base_url}): {e}"))?
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            bail!("GPT-SoVITS synthesis failed ({status}): {err_body}");
        }

        let bytes = resp.bytes().await?;
        if bytes.is_empty() {
            bail!("GPT-SoVITS returned empty audio data");
        }
        Ok(bytes.to_vec())
    }
}
