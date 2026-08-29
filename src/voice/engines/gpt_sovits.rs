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

    /// 根据参考音频文件名自动检索已收录的精准台词，防止空台词或错字导致合成效果劣化。
    fn resolve_known_prompt_text(audio_name_or_path: &str) -> Option<&'static str> {
        let leaf = Path::new(audio_name_or_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(audio_name_or_path)
            .trim();

        match leaf {
            // 傲娇经典（小盐核心声线）
            "4-03.wav" | "xiaoyan_tsundere_403.wav" => {
                Some("哼，今天就勉强允许你牵我的手好了，下不为例哦。")
            }
            // 温柔依偎
            "3-02.wav" | "xiaoyan_studio_clean.wav" | "xiaoyan_playful_302.wav" => {
                Some("靠近一点嘛，我又不会吃了你，除非你自己想被吃掉。")
            }
            // 调皮撩人
            "2-04（Y）.wav" | "2-04(Y).wav" | "xiaoyan_clear_204.wav" => {
                Some("别躲呀，看着我的眼睛，把你刚才想说的话再说一遍哦。")
            }
            // 元气自信
            "4-02.wav" | "xiaoyan_clear_402.wav" => {
                Some("我才没有特地打扮给你看呢，你千万别自作多情哦。")
            }
            // 独占女王
            "5-01.wav" | "xiaoyan_clear_501.wav" => {
                Some("你的眼睛里只能看着我一个人，听懂了吗？")
            }
            // 慵懒撒娇
            "3-06.wav" | "xiaoyan_gentle_306.wav" => {
                Some("嗯，好舒服，再陪我待五分钟，就五分钟，好不好？")
            }
            // 害羞脸红
            "2-01（n）.wav" | "2-01(n).wav" | "xiaoyan_clear_201.wav" => {
                Some("怎么再看我一眼就脸红啊，胆子这么小，以后可怎么办呀？")
            }
            // 甜美宠溺
            "2-02（Y）.wav" | "2-02(Y).wav" | "xiaoyan_sweet.wav" | "xiaoyan_ref.wav" | "sample_sweet.wav" => {
                Some("乖孩子叫声，自己来听听，说不定我就满足你的愿望呢。")
            }
            "4-01.wav" => Some("谁、谁让你看我这么近的，笨蛋，快把头转过去啊！"),
            "5-02.wav" => Some("躲去哪里都没有用的哦，你整个人早就是我的啦。"),
            "5-04.wav" => Some("为什么要看着别人呢？明明只要看着我，就足够了呀。"),
            "3-01.wav" => Some("嗯，好困啊，过来给我抱一下，不然今天不准你走。"),
            "3-03.wav" => Some("真是拿你没办法，过来，让我靠一会啊。"),
            "3-04.wav" => Some("话说得有点多了，头好晕啊，你要负责扶好我哦。"),
            "3-05.wav" => Some("别总看手机啊，我难道还没有屏幕好看吗？"),
            "2-03（Y）.wav" | "2-03(Y).wav" => Some("嘴上说着不要，身体倒是挺诚实的嘛，嗯？"),
            "2-05（Y-ns）.wav" | "2-05(Y-ns).wav" => Some("表现得这么乖，是想向我讨什么奖励吗？"),
            "2-06（Y-Y）.wav" | "2-06(Y-Y).wav" => Some("真是个不让人省心的小家伙，过来，坐到我身边来。"),
            "T-01.wav" => Some("真是败给你了。"),
            "T-02.wav" => Some("喂，你手往哪里放呢？"),
            "T-03.wav" => Some("怎么，不认得我了？"),
            "T-04.wav" => Some("今晚留下来陪我吧。"),
            "T-05.wav" => Some("嘘，别说话，吻我。"),
            _ => None,
        }
    }

    /// 解析参考音频的本地完整绝对路径，供 GPT-SoVITS 本地进程读取。
    fn resolve_ref_audio_path(&self, raw: Option<&str>) -> String {
        let clean = raw.map(|p| p.trim().trim_start_matches("local:")).unwrap_or("");

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

        if !clean.is_empty() {
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
        }

        // 默认按高品质声线优先级查找可用音频（优先匹配优质傲娇/温柔小盐录音）
        let preferred_defaults = [
            "4-03.wav",
            "xiaoyan_tsundere_403.wav",
            "3-02.wav",
            "xiaoyan_studio_clean.wav",
            "2-04（Y）.wav",
            "xiaoyan_clear_204.wav",
            "4-02.wav",
            "5-01.wav",
        ];

        for pref in &preferred_defaults {
            let candidate_dirs = [
                PathBuf::from("voices"),
                PathBuf::from("../voices"),
                PathBuf::from("../GPT-SoVITS-v2pro-20250604-nvidia50/audio_dataset"),
                PathBuf::from("GPT-SoVITS-v2pro-20250604-nvidia50/audio_dataset"),
            ];
            for dir in &candidate_dirs {
                let cand = dir.join(pref);
                if cand.exists() {
                    if let Ok(abs) = std::fs::canonicalize(&cand) {
                        return normalize_str(&abs);
                    }
                    return normalize_str(&cand);
                }
            }
        }

        // 兜底：查找 voices/ 目录下第一个有效音频
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

        // 语言优先默认采用 auto（智能识别中英混合），避免中文模式下读到英文单词破音
        let text_lang = config.text_lang.as_deref().unwrap_or("auto");
        let prompt_lang = config.prompt_lang.as_deref().unwrap_or("zh");

        // 自动补齐已知音频的高精度台词，避免空 Prompt Text 导致音质断崖式下降
        let effective_prompt_text = match config.prompt_text.as_deref() {
            Some(t) if !t.trim().is_empty() => t.trim().to_string(),
            _ => Self::resolve_known_prompt_text(&ref_audio)
                .unwrap_or("哼，今天就勉强允许你牵我的手好了，下不为例哦。")
                .to_string(),
        };

        let speed_factor: f64 = {
            let clean = config.rate.trim().trim_end_matches('%');
            clean.parse::<f64>().ok().map(|pct| (100.0 + pct) / 100.0)
        }
        .unwrap_or(1.0)
        .clamp(0.6, 1.8);

        // 采样参数调优（默认 temperature 0.80, top_k 5，显著提升音色稳定性，消除发飘与电音）
        let temperature = config.temperature.unwrap_or(0.80).clamp(0.3, 1.3);
        let top_k = config.top_k.unwrap_or(5).clamp(1, 30);
        let top_p = config.top_p.unwrap_or(1.0).clamp(0.5, 1.0);
        let repetition_penalty = config.repetition_penalty.unwrap_or(1.35).clamp(1.0, 2.0);

        let text_split_method = config
            .text_split_method
            .as_deref()
            .unwrap_or_else(|| {
                if clean_text.chars().count() > 50 {
                    "cut5"
                } else {
                    "cut0"
                }
            });

        let mut text_to_send = clean_text.to_string();
        if !text_to_send.ends_with(|c| "。！？!?.,;；…~".contains(c)) {
            text_to_send.push('。');
        }

        let body = json!({
            "text": text_to_send,
            "text_lang": text_lang,
            "ref_audio_path": ref_audio,
            "prompt_text": effective_prompt_text,
            "prompt_lang": prompt_lang,
            "text_split_method": text_split_method,
            "top_k": top_k,
            "top_p": top_p,
            "temperature": temperature,
            "speed_factor": speed_factor,
            "speed": speed_factor,
            "repetition_penalty": repetition_penalty,
            "sample_steps": 32,
            "fragment_interval": 0.25,
            "split_bucket": true,
            "parallel_infer": true,
            "media_type": "wav"
        });

        let start_time = std::time::Instant::now();
        tracing::info!(
            provider = "gpt_sovits",
            endpoint = %base_url,
            text_len = clean_text.len(),
            "GPT-SoVITS voice synthesis request started"
        );

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
                        let elapsed_ms = start_time.elapsed().as_millis();
                        tracing::info!(
                            provider = "gpt_sovits",
                            status = 200,
                            elapsed_ms = %elapsed_ms,
                            attempt = attempt,
                            audio_bytes = bytes.len(),
                            "GPT-SoVITS voice synthesis succeeded in {}ms",
                            elapsed_ms
                        );
                        return Ok(bytes.to_vec());
                    }

                    let err_body = resp.text().await.unwrap_or_default();
                    let elapsed_ms = start_time.elapsed().as_millis();
                    tracing::warn!(
                        provider = "gpt_sovits",
                        status = %status.as_u16(),
                        elapsed_ms = %elapsed_ms,
                        attempt = attempt,
                        "GPT-SoVITS attempt {} failed ({status}): {err_body}",
                        attempt
                    );
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
                                    let elapsed_ms = start_time.elapsed().as_millis();
                                    tracing::info!(
                                        provider = "gpt_sovits",
                                        status = 200,
                                        elapsed_ms = %elapsed_ms,
                                        attempt = attempt,
                                        audio_bytes = bytes.len(),
                                        "GPT-SoVITS voice synthesis (fallback) succeeded in {}ms",
                                        elapsed_ms
                                    );
                                    return Ok(bytes.to_vec());
                                }
                            }
                        }
                    }

                    let elapsed_ms = start_time.elapsed().as_millis();
                    tracing::warn!(
                        provider = "gpt_sovits",
                        elapsed_ms = %elapsed_ms,
                        attempt = attempt,
                        "Connection error to GPT-SoVITS ({base_url}) on attempt {}: {e}",
                        attempt
                    );
                    last_error = Some(anyhow::anyhow!("Connection error to GPT-SoVITS ({base_url}): {e}"));
                }
            }
        }

        let elapsed_ms = start_time.elapsed().as_millis();
        tracing::error!(
            provider = "gpt_sovits",
            elapsed_ms = %elapsed_ms,
            "GPT-SoVITS voice synthesis failed after {max_attempts} attempts: {:?}",
            last_error
        );
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to connect to GPT-SoVITS API ({base_url})")))
    }
}

