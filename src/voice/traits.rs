//! 语音引擎抽象与多引擎分发。

use crate::voice::engines::cosyvoice::CosyVoiceEngine;
use crate::voice::engines::custom_api::CustomHttpTtsEngine;
use crate::voice::engines::edge_tts::EdgeTtsEngine;
use crate::voice::engines::gpt_sovits::GptSovitsEngine;
use crate::voice::engines::openai_tts::OpenAiTtsEngine;
use crate::voice::types::{VoiceConfig, VoiceEngineKind};
use anyhow::Result;

#[derive(Clone)]
pub enum VoiceEngine {
    EdgeTts(EdgeTtsEngine),
    GptSovits(GptSovitsEngine),
    CosyVoice(CosyVoiceEngine),
    OpenAi(OpenAiTtsEngine),
    CustomHttp(CustomHttpTtsEngine),
}

impl VoiceEngine {
    pub fn new(kind: VoiceEngineKind) -> Self {
        match kind {
            VoiceEngineKind::EdgeTts => Self::EdgeTts(EdgeTtsEngine::new()),
            VoiceEngineKind::GptSovits => Self::GptSovits(GptSovitsEngine::new()),
            VoiceEngineKind::CosyVoice => Self::CosyVoice(CosyVoiceEngine::new()),
            VoiceEngineKind::OpenAi => Self::OpenAi(OpenAiTtsEngine::new()),
            VoiceEngineKind::CustomHttp => Self::CustomHttp(CustomHttpTtsEngine::new()),
        }
    }

    pub async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Vec<u8>> {
        match self {
            Self::EdgeTts(engine) => engine.synthesize(text, config).await,
            Self::GptSovits(engine) => engine.synthesize(text, config).await,
            Self::CosyVoice(engine) => engine.synthesize(text, config).await,
            Self::OpenAi(engine) => engine.synthesize(text, config).await,
            Self::CustomHttp(engine) => engine.synthesize(text, config).await,
        }
    }
}
