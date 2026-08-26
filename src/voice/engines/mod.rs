pub mod cosyvoice;
pub mod custom_api;
pub mod edge_tts;
pub mod gpt_sovits;
pub mod openai_tts;

pub use cosyvoice::CosyVoiceEngine;
pub use custom_api::CustomHttpTtsEngine;
pub use edge_tts::EdgeTtsEngine;
pub use gpt_sovits::GptSovitsEngine;
pub use openai_tts::OpenAiTtsEngine;
