//! 纯 Rust 微软 Edge-TTS 客户端实现。
//!
//! 通过 WebSocket 协议与 Edge ReadAloud 语音服务交互，
//! 支持 SSML 音调（Pitch）、语速（Rate）与音量（Volume）精确调制。

use crate::voice::types::VoiceConfig;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

const EDGE_TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const EDGE_WSS_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";
const WIN_EPOCH: i64 = 11644473600;
const SEC_MS_GEC_VERSION: &str = "1-143.0.3650.75";

fn generate_sec_ms_gec() -> String {
    let unix_now = Utc::now().timestamp();
    let mut ticks = unix_now + WIN_EPOCH;
    ticks -= ticks % 300;
    let ticks_100ns = (ticks as u64) * 10_000_000;
    let ticks_str = format!("{ticks_100ns}{EDGE_TRUSTED_CLIENT_TOKEN}");
    let mut hasher = Sha256::new();
    hasher.update(ticks_str.as_bytes());
    let result = hasher.finalize();
    hex::encode(result).to_uppercase()
}

#[derive(Clone, Default)]
pub struct EdgeTtsEngine;

impl EdgeTtsEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Vec<u8>> {
        let clean_text = text.trim();
        if clean_text.is_empty() {
            return Ok(Vec::new());
        }

        let start_time = std::time::Instant::now();
        tracing::info!(
            provider = "edge_tts",
            voice = %config.voice,
            text_len = clean_text.len(),
            "Edge-TTS voice synthesis request started"
        );

        let connection_id = format!("{:032x}", rand::random::<u128>());
        let request_id = format!("{:032x}", rand::random::<u128>());
        let sec_ms_gec = generate_sec_ms_gec();
        let url = format!(
            "{}?TrustedClientToken={}&Sec-MS-GEC={}&Sec-MS-GEC-Version={}&ConnectionId={}",
            EDGE_WSS_URL, EDGE_TRUSTED_CLIENT_TOKEN, sec_ms_gec, SEC_MS_GEC_VERSION, connection_id
        );

        let mut request = url
            .into_client_request()
            .context("Failed to construct Edge TTS websocket request")?;

        let headers = request.headers_mut();
        headers.insert("Pragma", "no-cache".parse()?);
        headers.insert("Cache-Control", "no-cache".parse()?);
        headers.insert(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0".parse()?,
        );
        headers.insert(
            "Origin",
            "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".parse()?,
        );
        headers.insert("Accept-Encoding", "gzip, deflate, br, zstd".parse()?);
        headers.insert("Accept-Language", "en-US,en;q=0.9".parse()?);

        let mut ws_stream = None;
        let mut last_err = None;
        for _ in 0..3 {
            match connect_async(request.clone()).await {
                Ok((stream, _)) => {
                    ws_stream = Some(stream);
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                }
            }
        }

        let ws_stream = ws_stream.ok_or_else(|| {
            anyhow::anyhow!("Failed to connect to Edge TTS WebSocket: {:?}", last_err)
        })?;

        let (mut write, mut read) = ws_stream.split();

        // 1. 发送配置帧
        let config_message = "Content-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"true\"},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}";
        write.send(Message::Text(config_message.into())).await?;

        // 2. 构造 SSML 语音合成请求帧
        let escaped_text = escape_ssml_text(clean_text);
        let lang = {
            let parts: Vec<&str> = config.voice.split('-').collect();
            if parts.len() >= 2 {
                format!("{}-{}", parts[0], parts[1])
            } else {
                "zh-CN".to_string()
            }
        };

        let ssml = format!(
            "<speak version=\"1.0\" xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"https://www.w3.org/2001/mstts\" xml:lang=\"{lang}\"><voice name=\"{}\"><prosody pitch=\"{}\" rate=\"{}\" volume=\"{}\">{}</prosody></voice></speak>",
            config.voice, config.pitch, config.rate, config.volume, escaped_text
        );

        let ssml_message = format!(
            "X-RequestId:{request_id}\r\nContent-Type:application/ssml+xml\r\nPath:ssml\r\n\r\n{ssml}"
        );
        write.send(Message::Text(ssml_message.into())).await?;

        // 3. 接收音频流
        let mut audio_buffer = Vec::new();
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(txt)) => {
                    if txt.as_str().contains("Path:turn.end") {
                        break;
                    }
                }
                Ok(Message::Binary(bin)) => {
                    if bin.len() >= 2 {
                        let header_len = u16::from_be_bytes([bin[0], bin[1]]) as usize;
                        let audio_start = 2 + header_len;
                        if bin.len() > audio_start {
                            audio_buffer.extend_from_slice(&bin[audio_start..]);
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(err) => {
                    tracing::warn!("Edge TTS WebSocket error: {err}");
                    break;
                }
                _ => {}
            }
        }

        if audio_buffer.is_empty() {
            let elapsed_ms = start_time.elapsed().as_millis();
            tracing::error!(
                provider = "edge_tts",
                voice = %config.voice,
                elapsed_ms = %elapsed_ms,
                "Edge-TTS returned empty audio buffer"
            );
            bail!("Edge TTS returned empty audio buffer");
        }

        let elapsed_ms = start_time.elapsed().as_millis();
        tracing::info!(
            provider = "edge_tts",
            voice = %config.voice,
            status = 200,
            elapsed_ms = %elapsed_ms,
            audio_bytes = audio_buffer.len(),
            "Edge-TTS voice synthesis completed in {}ms",
            elapsed_ms
        );

        Ok(audio_buffer)
    }
}

fn escape_ssml_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_ssml() {
        assert_eq!(
            escape_ssml_text("Miyu & <Neuro> 'say' \"hello\""),
            "Miyu &amp; &lt;Neuro&gt; &apos;say&apos; &quot;hello&quot;"
        );
    }
}
