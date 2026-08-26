//! Miyu 智能流式语音交互系统。
//!
//! 包含流式智能断句 (SentenceChunker)、纯 Rust 微软 Edge-TTS 客户端 (支持 Neuro 同款变调)、
//! 异步播放与抢占打断控制器 (VoicePlayer) 及流水线调度总门面 (VoiceService)。

pub mod chunker;
pub mod engines;
pub mod player;
pub mod service;
pub mod traits;
pub mod types;

#[cfg(test)]
mod tests;

pub use chunker::SentenceChunker;
pub use player::VoicePlayer;
pub use service::VoiceService;
pub use traits::VoiceEngine;
pub use types::{VoiceConfig, VoiceEngineKind, VoicePreset};
