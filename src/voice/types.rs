use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceEngineKind {
    EdgeTts,
    GptSovits,
    CosyVoice,
    OpenAi,
    CustomHttp,
}

impl Default for VoiceEngineKind {
    fn default() -> Self {
        Self::EdgeTts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VoicePreset {
    /// Neuro-sama classic cute voice (Ashley + Pitch +45Hz + Rate +8%)
    NeuroClassic,
    /// Evil Neuro sarcastic voice (Sara + Pitch +15Hz + Rate -5%)
    NeuroEvil,
    /// Miyu Chinese sweet voice (Xiaoxiao + Pitch +30Hz + Rate +5%)
    MiyuChinese,
    /// Custom settings
    Custom,
}

impl VoicePreset {
    pub fn apply_to(&self, config: &mut VoiceConfig) {
        match self {
            VoicePreset::NeuroClassic => {
                config.engine = VoiceEngineKind::EdgeTts;
                config.voice = "en-US-AshleyNeural".to_string();
                config.pitch = "+45Hz".to_string();
                config.rate = "+8%".to_string();
                config.volume = "+0%".to_string();
            }
            VoicePreset::NeuroEvil => {
                config.engine = VoiceEngineKind::EdgeTts;
                config.voice = "en-US-SaraNeural".to_string();
                config.pitch = "+15Hz".to_string();
                config.rate = "-5%".to_string();
                config.volume = "+0%".to_string();
            }
            VoicePreset::MiyuChinese => {
                config.engine = VoiceEngineKind::EdgeTts;
                config.voice = "zh-CN-XiaoxiaoNeural".to_string();
                config.pitch = "+30Hz".to_string();
                config.rate = "+5%".to_string();
                config.volume = "+0%".to_string();
            }
            VoicePreset::Custom => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub engine: VoiceEngineKind,

    #[serde(default)]
    pub preset: Option<VoicePreset>,

    #[serde(default = "default_voice")]
    pub voice: String,

    #[serde(default = "default_pitch")]
    pub pitch: String,

    #[serde(default = "default_rate")]
    pub rate: String,

    #[serde(default = "default_volume")]
    pub volume: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Reference audio file path or filename for voice cloning (e.g. "voices/sample.wav")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_audio: Option<String>,

    /// Reference audio prompt text spoken in the reference audio
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_text: Option<String>,

    /// Language of the reference audio (e.g. "zh", "ja", "en", "auto")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_lang: Option<String>,

    /// Target text synthesis language (e.g. "zh", "ja", "en", "auto")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_lang: Option<String>,

    /// Whether to read code blocks (```...```) out loud. Defaults to false.
    #[serde(default)]
    pub read_code_blocks: bool,
}

fn default_voice() -> String {
    "zh-CN-XiaoxiaoNeural".to_string()
}

fn default_pitch() -> String {
    "+30Hz".to_string()
}

fn default_rate() -> String {
    "+5%".to_string()
}

fn default_volume() -> String {
    "+0%".to_string()
}

impl Default for VoiceConfig {
    fn default() -> Self {
        let mut config = Self {
            enabled: false,
            engine: VoiceEngineKind::EdgeTts,
            preset: Some(VoicePreset::MiyuChinese),
            voice: default_voice(),
            pitch: default_pitch(),
            rate: default_rate(),
            volume: default_volume(),
            endpoint: None,
            api_key: None,
            prompt_audio: None,
            prompt_text: None,
            prompt_lang: None,
            text_lang: None,
            read_code_blocks: false,
        };
        VoicePreset::MiyuChinese.apply_to(&mut config);
        config
    }
}

impl VoiceConfig {
    pub fn is_default(&self) -> bool {
        !self.enabled
            && self.engine == VoiceEngineKind::EdgeTts
            && self.preset == Some(VoicePreset::MiyuChinese)
            && self.endpoint.is_none()
            && self.api_key.is_none()
            && self.prompt_audio.is_none()
            && self.prompt_text.is_none()
            && !self.read_code_blocks
    }
}
