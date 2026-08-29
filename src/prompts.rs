use base64::Engine;

include!(concat!(env!("OUT_DIR"), "/default_natria_prompt.rs"));

pub const MEME_DESCRIPTION_PROMPT: &str = include_str!("prompts/meme-description.md");
pub const COMPACT_SYSTEM_PROMPT: &str = include_str!("prompts/compact.md");

fn decode_embedded_prompt(encoded: &str) -> String {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("embedded default prompt must be valid base64");
    let decoded = bytes
        .into_iter()
        .enumerate()
        .map(|(index, byte)| byte ^ PROMPT_MASK[index % PROMPT_MASK.len()])
        .collect::<Vec<_>>();
    String::from_utf8(decoded).expect("embedded default prompt must be valid utf-8")
}

pub fn default_system_prompt() -> String {
    decode_embedded_prompt(OBFUSCATED_DEFAULT_SYSTEM_PROMPT)
}

/// 默认 Natria 人格的内置防失忆提示(A/B 实测定稿文本)。用户在
/// hints/default.md 写了自己的内容时被覆盖。
pub fn default_natria_hint() -> String {
    decode_embedded_prompt(OBFUSCATED_DEFAULT_NATRIA_HINT)
}

/// 兼容别名
#[inline]
pub fn default_miyu_hint() -> String {
    default_natria_hint()
}

/// 默认 Natria 人格的内置预设对话(begin_dialogs)。用户在
/// dialogs/default.md 写了自己的内容时被覆盖。
pub fn default_natria_dialogs() -> String {
    decode_embedded_prompt(OBFUSCATED_DEFAULT_NATRIA_DIALOGS)
}

/// 兼容别名
#[inline]
pub fn default_miyu_dialogs() -> String {
    default_natria_dialogs()
}
