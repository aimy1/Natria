//! GPT-SoVITS 本地零样本声音克隆 TTS 引擎客户端。
//!
//! 对接本地 GPT-SoVITS API 服务（默认端口 9880），
//! 传入参考音频（Prompt Audio）与参考文本（Prompt Text），实现任意文本的声音克隆朗读。
//! 内置长连接池维持、自动保活（Keep-Alive）与断线重试机制，防止意外断链。

use crate::voice::types::VoiceConfig;
use anyhow::{bail, Result};
use reqwest::Client;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;

static INFER_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone)]
pub struct GptSovitsEngine {
    client: Client,
}

impl GptSovitsEngine {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(90))
                .connect_timeout(Duration::from_secs(10))
                .tcp_keepalive(Some(Duration::from_secs(30)))
                .pool_idle_timeout(Some(Duration::from_secs(120)))
                .pool_max_idle_per_host(8)
                .build()
                .unwrap_or_default(),
        }
    }

    /// 解析参考音频的本地完整绝对路径，供 GPT-SoVITS 本地进程读取。
    fn resolve_ref_audio_path(&self, raw: Option<&str>) -> String {
        let clean = raw.map(|p| p.trim().trim_start_matches("local:")).unwrap_or("");
        if clean.is_empty() {
            return String::new();
        }

        let normalize_str = |p: &Path| -> String {
            let s = p.to_string_lossy().to_string();
            if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
                format!(r"\\{}", stripped)
            } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
                stripped.to_string()
            } else {
                s
            }
        };

        let raw_path = Path::new(clean);
        if raw_path.is_absolute() && raw_path.exists() {
            if let Ok(canon) = std::fs::canonicalize(raw_path) {
                return normalize_str(&canon);
            }
            return normalize_str(raw_path);
        }

        // 尝试从多个常见候选目录查找指定音频文件
        let candidate_dirs = [
            PathBuf::from("voices"),
            PathBuf::from("../voices"),
            PathBuf::from("GPT-SoVITS-v2pro-20250604-nvidia50/audio_dataset"),
            PathBuf::from("../GPT-SoVITS-v2pro-20250604-nvidia50/audio_dataset"),
            PathBuf::from("audio_dataset"),
        ];

        for dir in &candidate_dirs {
            let cand = dir.join(clean);
            if cand.exists() {
                if let Ok(abs) = std::fs::canonicalize(&cand) {
                    return normalize_str(&abs);
                }
                return normalize_str(&cand);
            }
        }

        // 查找 voices/ 目录下第一个有效音频作为后备
        if let Ok(entries) = std::fs::read_dir("voices") {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if ext.eq_ignore_ascii_case("wav") || ext.eq_ignore_ascii_case("mp3") {
                            if let Ok(abs) = std::fs::canonicalize(&p) {
                                return normalize_str(&abs);
                            }
                            return normalize_str(&p);
                        }
                    }
                }
            }
        }

        clean.to_string()
    }

    pub async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Vec<u8>> {
        let clean_text = text.trim();
        if clean_text.is_empty() {
            return Ok(Vec::new());
        }

        // 排队锁：防止前端流式输出并发轰炸导致 GPT-SoVITS 崩溃或断链
        let _guard = INFER_MUTEX.lock().await;

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

        let speed_factor: f64 = {
            let clean = config.rate.trim().trim_end_matches('%');
            clean.parse::<f64>().ok().map(|pct| (100.0 + pct) / 100.0)
        }
        .unwrap_or(1.0)
        .clamp(0.6, 1.8);

        let mut text_to_send = clean_text.to_string();
        if !text_to_send.ends_with(|c| "。！？!?.,;；…~".contains(c)) {
            text_to_send.push('。');
        }

        let body = json!({
            "text": text_to_send,
            "text_lang": text_lang,
            "ref_audio_path": ref_audio,
            "prompt_text": prompt_text,
            "prompt_lang": prompt_lang,
            "text_split_method": "cut0",
            "top_k": 15,
            "top_p": 1.0,
            "temperature": 1.0,
            "speed_factor": speed_factor,
            "repetition_penalty": 1.35,
            "sample_steps": 32,
            "media_type": "wav"
        });

        // 自动重试机制：针对临时空闲断开、瞬时波动重试最多 3 次
        let max_attempts = 3;
        let mut last_error: Option<anyhow::Error> = None;

        for attempt in 1..=max_attempts {
            if attempt > 1 {
                tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
            }

            let mut req = self.client.post(&url).json(&body);
            if let Some(key) = &config.api_key {
                req = req.bearer_auth(key);
            }

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let bytes = resp.bytes().await?;
                        if bytes.is_empty() {
                            bail!("GPT-SoVITS API returned empty audio stream");
                        }
                        return Ok(bytes.to_vec());
                    }

                    let err_body = resp.text().await.unwrap_or_default();
                    if status.is_server_error() && attempt < max_attempts {
                        last_error = Some(anyhow::anyhow!("GPT-SoVITS HTTP {status}: {err_body}"));
                        continue;
                    }
                    bail!("GPT-SoVITS synthesis failed ({status}): {err_body}");
                }
                Err(e) => {
                    // 若 /tts 404/连接失败，且是首轮尝试，尝试向根路径 fallback
                    if attempt == 1 {
                        let fallback_url = format!("{base_url}/");
                        let mut fallback_req = self.client.post(&fallback_url).json(&body);
                        if let Some(key) = &config.api_key {
                            fallback_req = fallback_req.bearer_auth(key);
                        }
                        if let Ok(resp) = fallback_req.send().await {
                            if resp.status().is_success() {
                                let bytes = resp.bytes().await?;
                                if !bytes.is_empty() {
                                    return Ok(bytes.to_vec());
                                }
                            }
                        }
                    }

                    last_error = Some(anyhow::anyhow!("Connection error to GPT-SoVITS ({base_url}): {e}"));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to connect to GPT-SoVITS API ({base_url})")))
    }
}

