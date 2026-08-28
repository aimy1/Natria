//! 网页端语音合成（TTS）与本地语音文件管理接口。
//!
//! 供 WebUI 前端按需合成文本为自然语音 MP3，支持自定义音色、音调、语速，
//! 以及本地语音音频文件（.mp3, .wav, .ogg, .flac 等）的列表、上传、播放与删除。

use crate::voice::types::VoiceEngineKind;
use crate::web::*;
use axum::body::{Body, Bytes};
use axum::extract::Path;
use axum::http::header::{CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use std::path::{Path as StdPath, PathBuf};

#[derive(Deserialize)]
pub(in crate::web) struct SynthesizeVoiceRequest {
    pub(in crate::web) text: String,
    #[serde(default)]
    pub(in crate::web) engine: Option<VoiceEngineKind>,
    #[serde(default)]
    pub(in crate::web) endpoint: Option<String>,
    #[serde(default)]
    pub(in crate::web) api_key: Option<String>,
    #[serde(default)]
    pub(in crate::web) prompt_audio: Option<String>,
    #[serde(default)]
    pub(in crate::web) prompt_text: Option<String>,
    #[serde(default)]
    pub(in crate::web) prompt_lang: Option<String>,
    #[serde(default)]
    pub(in crate::web) text_lang: Option<String>,
    #[serde(default)]
    pub(in crate::web) voice: Option<String>,
    #[serde(default)]
    pub(in crate::web) pitch: Option<String>,
    #[serde(default)]
    pub(in crate::web) rate: Option<String>,
    #[serde(default)]
    pub(in crate::web) volume: Option<String>,
}

#[derive(Serialize)]
pub(in crate::web) struct VoiceFileItem {
    pub(in crate::web) name: String,
    pub(in crate::web) size_bytes: u64,
    pub(in crate::web) size_formatted: String,
    pub(in crate::web) ext: String,
    pub(in crate::web) modified_at: Option<String>,
    pub(in crate::web) url: String,
}

pub(in crate::web) fn get_voices_dir(state: &DaemonState) -> PathBuf {
    let local = PathBuf::from("voices");
    if local.exists() {
        return local;
    }
    let data_voices = state.paths.data_dir.join("voices");
    if !data_voices.exists() {
        let _ = std::fs::create_dir_all(&data_voices);
    }
    let _ = std::fs::create_dir_all(&local);
    local
}

pub(in crate::web) fn sanitize_voice_filename(name: &str) -> std::result::Result<String, ApiError> {
    let clean = name.trim().replace('\\', "/");
    let leaf = clean.split('/').last().unwrap_or("").trim();
    if leaf.is_empty() || leaf.starts_with('.') || leaf.contains("..") {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid voice file name"));
    }
    let valid_ext = ["mp3", "wav", "ogg", "flac", "m4a", "aac", "opus", "wma"];
    let ext = StdPath::new(leaf)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if !valid_ext.contains(&ext.as_str()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("unsupported audio format .{ext} (supported: {})", valid_ext.join(", ")),
        ));
    }
    Ok(leaf.to_string())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub(in crate::web) fn audio_mime(path: &StdPath) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") | Some("opus") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("m4a") | Some("aac") => "audio/mp4",
        _ => "application/octet-stream",
    }
}

pub(in crate::web) async fn list_voice_files_http(
    State(state): State<DaemonState>,
    headers: HeaderMap,
) -> std::result::Result<Json<serde_json::Value>, ApiError> {
    require_auth(&headers, &state)?;
    let voices_dir = get_voices_dir(&state);
    let mut files = Vec::new();

    if let Ok(mut entries) = tokio::fs::read_dir(&voices_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            let valid_ext = ["mp3", "wav", "ogg", "flac", "m4a", "aac", "opus", "wma"];
            if !valid_ext.contains(&ext.as_str()) {
                continue;
            }
            let meta = match tokio::fs::metadata(&path).await {
                Ok(m) => m,
                Err(_) => continue,
            };
            let size_bytes = meta.len();
            let modified_at = meta
                .modified()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
            files.push(VoiceFileItem {
                url: format!("/api/voice/files/{name}"),
                size_formatted: format_bytes(size_bytes),
                name,
                size_bytes,
                ext,
                modified_at,
            });
        }
    }

    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(Json(json!({ "files": files })))
}

pub(in crate::web) async fn upload_voice_file_http(
    State(state): State<DaemonState>,
    headers: HeaderMap,
    body: Bytes,
) -> std::result::Result<Json<serde_json::Value>, ApiError> {
    require_auth(&headers, &state)?;
    if body.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "empty file body"));
    }
    if body.len() > 50 * 1024 * 1024 {
        return Err(ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "file exceeds 50MB limit"));
    }
    let filename_header = headers
        .get("x-natria-filename")
        .or_else(|| headers.get("x-miyu-filename"))
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "x-natria-filename header is required"))?;
    let decoded = urlencoding::decode(filename_header)
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid urlencoded filename"))?;
    let safe_name = sanitize_voice_filename(&decoded)?;

    let voices_dir = get_voices_dir(&state);
    let target_path = voices_dir.join(&safe_name);
    tokio::fs::write(&target_path, &body)
        .await
        .map_err(ApiError::internal)?;

    let size_bytes = body.len() as u64;
    Ok(Json(json!({
        "ok": true,
        "name": safe_name,
        "size_bytes": size_bytes,
        "size_formatted": format_bytes(size_bytes),
        "url": format!("/api/voice/files/{safe_name}"),
    })))
}

pub(in crate::web) async fn get_voice_file_http(
    State(state): State<DaemonState>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> std::result::Result<Response, ApiError> {
    require_auth(&headers, &state)?;
    let safe_name = sanitize_voice_filename(&filename)?;
    let voices_dir = get_voices_dir(&state);
    let path = voices_dir.join(&safe_name);
    if !path.is_file() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "voice file not found"));
    }

    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "voice file not found"))?;
    let total = metadata.len();
    let mime = audio_mime(&path);

    let range = headers
        .get(RANGE)
        .and_then(|value| value.to_str().ok())
        .map(|value| parse_byte_range(value, total));

    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "voice file not found"))?;

    let (status, start, end) = match range {
        None => (StatusCode::OK, 0, total.saturating_sub(1)),
        Some(Some((start, end))) => (StatusCode::PARTIAL_CONTENT, start, end),
        Some(None) => {
            let mut response = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            response.headers_mut().insert(
                CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes */{total}")).unwrap(),
            );
            return Ok(response);
        }
    };
    let length = if total == 0 { 0 } else { end - start + 1 };
    use tokio::io::AsyncSeekExt;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(ApiError::internal)?;
    let stream = tokio_util::io::ReaderStream::new(tokio::io::AsyncReadExt::take(file, length));
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = status;
    let resp_headers = response.headers_mut();
    resp_headers.insert(CONTENT_TYPE, HeaderValue::from_static(mime));
    resp_headers.insert(CONTENT_LENGTH, HeaderValue::from(length));
    resp_headers.insert(
        axum::http::header::ACCEPT_RANGES,
        HeaderValue::from_static("bytes"),
    );
    if status == StatusCode::PARTIAL_CONTENT {
        resp_headers.insert(
            CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{total}")).unwrap(),
        );
    }
    resp_headers.insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    Ok(response)
}

pub(in crate::web) async fn delete_voice_file_http(
    State(state): State<DaemonState>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> std::result::Result<Json<serde_json::Value>, ApiError> {
    require_auth(&headers, &state)?;
    let safe_name = sanitize_voice_filename(&filename)?;
    let voices_dir = get_voices_dir(&state);
    let path = voices_dir.join(&safe_name);
    if !path.is_file() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "voice file not found"));
    }
    tokio::fs::remove_file(&path)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true, "deleted": safe_name })))
}

pub(in crate::web) async fn synthesize_voice_http(
    State(state): State<DaemonState>,
    headers: HeaderMap,
    Json(request): Json<SynthesizeVoiceRequest>,
) -> std::result::Result<Response, ApiError> {
    require_auth(&headers, &state)?;

    let text = request.text.trim();
    if text.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "text cannot be empty",
        ));
    }

    let mut config = state.manager.lock().unwrap().config.voice.clone();
    config.enabled = true;
    if let Some(engine) = request.engine {
        config.engine = engine;
    }
    if let Some(endpoint) = request.endpoint {
        config.endpoint = Some(endpoint);
    }
    if let Some(api_key) = request.api_key {
        config.api_key = Some(api_key);
    }
    if let Some(prompt_audio) = request.prompt_audio {
        config.prompt_audio = Some(prompt_audio);
    }
    if let Some(prompt_text) = request.prompt_text {
        config.prompt_text = Some(prompt_text);
    }
    if let Some(prompt_lang) = request.prompt_lang {
        config.prompt_lang = Some(prompt_lang);
    }
    if let Some(text_lang) = request.text_lang {
        config.text_lang = Some(text_lang);
    }
    if let Some(voice) = request.voice {
        config.voice = voice;
    }
    if let Some(pitch) = request.pitch {
        config.pitch = pitch;
    }
    if let Some(rate) = request.rate {
        config.rate = rate;
    }
    if let Some(volume) = request.volume {
        config.volume = volume;
    }

    if config.engine == VoiceEngineKind::EdgeTts && config.voice.starts_with("local:") {
        let leaf = config.voice.trim_start_matches("local:");
        if let Ok(safe_name) = sanitize_voice_filename(leaf) {
            let voices_dir = get_voices_dir(&state);
            let path = voices_dir.join(&safe_name);
            if path.is_file() {
                let data = tokio::fs::read(&path)
                    .await
                    .map_err(ApiError::internal)?;
                let mime = audio_mime(&path);
                let response = Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, mime)
                    .header("Cache-Control", "no-cache")
                    .body(Body::from(data))
                    .map_err(|err| ApiError::internal(anyhow::anyhow!("failed to construct response: {err}")))?;
                return Ok(response);
            }
        }
    }

    let started = std::time::Instant::now();
    tracing::info!(
        category = "voice",
        engine = ?config.engine,
        text_len = text.len(),
        "Voice synthesis requested: engine={:?} text_len={}",
        config.engine,
        text.len()
    );

    let engine = crate::voice::traits::VoiceEngine::new(config.engine);
    let audio_bytes = match engine.synthesize(text, &config).await {
        Ok(bytes) => {
            let elapsed_ms = started.elapsed().as_millis();
            tracing::info!(
                category = "voice",
                engine = ?config.engine,
                status = 200,
                elapsed_ms = %elapsed_ms,
                bytes = bytes.len(),
                "Voice synthesis finished in {}ms ({} bytes)",
                elapsed_ms,
                bytes.len()
            );
            bytes
        }
        Err(err) => {
            let elapsed_ms = started.elapsed().as_millis();
            tracing::error!(
                category = "voice",
                engine = ?config.engine,
                status = 500,
                elapsed_ms = %elapsed_ms,
                "Voice synthesis failed ({}ms): {err:#}",
                elapsed_ms
            );
            return Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("TTS synthesis failed: {err:#}"),
            ));
        }
    };

    let content_type = match config.engine {
        VoiceEngineKind::GptSovits | VoiceEngineKind::CosyVoice => "audio/wav",
        _ => "audio/mpeg",
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header("Cache-Control", "no-cache")
        .body(Body::from(audio_bytes))
        .map_err(|err| ApiError::internal(anyhow::anyhow!("failed to construct response: {err}")))?;

    Ok(response)
}
