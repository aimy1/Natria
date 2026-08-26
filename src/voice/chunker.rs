//! 流式断句与 Markdown 清洗器。
//!
//! 在 LLM 生成 token 时进行实时流式缓冲与分句，剥离 Markdown 代码块与特殊排版符号，
//! 按中英文标点智能切分短句，使得 TTS 引擎能以最低延迟合成发声。

#[derive(Debug, Clone)]
pub struct SentenceChunker {
    read_code_blocks: bool,
    buffer: String,
    in_code_block: bool,
}

impl SentenceChunker {
    pub fn new(read_code_blocks: bool) -> Self {
        Self {
            read_code_blocks,
            buffer: String::new(),
            in_code_block: false,
        }
    }

    /// 追加流式文本片段，返回所有已就绪的完整句子。
    pub fn push_delta(&mut self, delta: &str) -> Vec<String> {
        self.buffer.push_str(delta);
        self.extract_sentences(false)
    }

    /// 流式生成结束时刷新缓冲区中剩余的文本。
    pub fn finish(&mut self) -> Vec<String> {
        self.extract_sentences(true)
    }

    /// 提取就绪句子。
    fn extract_sentences(&mut self, is_final: bool) -> Vec<String> {
        let mut ready = Vec::new();

        while !self.buffer.is_empty() {
            // 处理代码块状态
            if !self.read_code_blocks {
                if let Some(fence_pos) = self.buffer.find("```") {
                    if !self.in_code_block {
                        // 代码块前的普通文本可以处理
                        let text_before = self.buffer[..fence_pos].to_string();
                        self.buffer = self.buffer[fence_pos + 3..].to_string();
                        self.in_code_block = true;
                        
                        let sents = split_into_clean_sentences(&text_before, true);
                        ready.extend(sents);
                        continue;
                    } else {
                        // 在代码块内，找到闭合 fence
                        self.buffer = self.buffer[fence_pos + 3..].to_string();
                        self.in_code_block = false;
                        continue;
                    }
                } else if self.in_code_block {
                    // 还在代码块内部且没有闭合 fence
                    if is_final {
                        self.buffer.clear();
                    }
                    break;
                }
            }

            if let Some((cut_idx, end_idx)) = find_next_sentence_cut(&self.buffer, is_final) {
                let segment = self.buffer[..cut_idx].to_string();
                self.buffer = self.buffer[end_idx..].to_string();
                let clean = clean_markdown_for_speech(&segment);
                if !clean.trim().is_empty() {
                    ready.push(clean);
                }
            } else {
                break;
            }
        }

        ready
    }
}

/// 查找下一个切句位置。
/// 返回 `Some((cut_point, next_start_point))`。
fn find_next_sentence_cut(text: &str, is_final: bool) -> Option<(usize, usize)> {
    if text.is_empty() {
        return None;
    }

    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let char_count = chars.len();

    // 强断句标点：句号、叹号、问号、分号、换行
    const STRONG_PUNCT: &[char] = &['。', '！', '!', '？', '?', '；', ';', '\n', '\r'];
    // 弱断句标点：逗号、冒号、顿号
    const WEAK_PUNCT: &[char] = &['，', ',', '：', ':', '、'];

    for (i, &(_byte_offset, ch)) in chars.iter().enumerate() {
        let is_strong = STRONG_PUNCT.contains(&ch);
        let is_weak = WEAK_PUNCT.contains(&ch);

        if is_strong {
            let next_byte = if i + 1 < char_count {
                chars[i + 1].0
            } else {
                text.len()
            };
            return Some((next_byte, next_byte));
        }

        // 如果句子已经累积了一定长度（如 18 个字符以上），遇到弱标点也可以切分，提升首包流式感
        if is_weak && i >= 18 {
            let next_byte = if i + 1 < char_count {
                chars[i + 1].0
            } else {
                text.len()
            };
            return Some((next_byte, next_byte));
        }
    }

    if is_final {
        Some((text.len(), text.len()))
    } else {
        None
    }
}

/// 移除文本中的各种动作描写（（...）、(...)、【...】、［...］）以及星号动作（*...*）
pub fn strip_action_descriptions(text: &str) -> String {
    let mut s = text.to_string();

    // 1. 递归剥离各类括号（支持多层嵌套）
    let open_chars = ['（', '(', '【', '［'];
    loop {
        let mut changed = false;
        let mut best_start = None;
        for (idx, ch) in s.char_indices() {
            if open_chars.contains(&ch) {
                best_start = Some((idx, ch));
            } else if let Some((start_idx, open_ch)) = best_start {
                let matches_close = match open_ch {
                    '（' => ch == '）',
                    '(' => ch == ')',
                    '【' => ch == '】',
                    '［' => ch == '］',
                    _ => false,
                };
                if matches_close {
                    s.replace_range(start_idx..idx + ch.len_utf8(), "");
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }

    // 2. 剥离星号动作描写，如 *脸红*、*叹了口气*
    while let Some(start) = s.find('*') {
        if let Some(end) = s[start + 1..].find('*') {
            s.replace_range(start..start + 1 + end + 1, "");
        } else {
            break;
        }
    }

    // 3. 去除首尾残留的脱落标点符号与空格
    let trimmed = s.trim();
    let cleaned = trimmed
        .trim_start_matches(|c| matches!(c, '，' | ',' | '、' | '；' | ';' | '：' | ':' | ' ' | '\t'))
        .trim_end_matches(|c| matches!(c, '，' | ',' | '、' | '；' | ';' | '：' | ':' | ' ' | '\t'));
    cleaned.to_string()
}

/// 将文本中的 Markdown 标记与动作描写清洗为适合语音朗读的纯文本。
pub fn clean_markdown_for_speech(raw: &str) -> String {
    let without_actions = strip_action_descriptions(raw);
    let mut text = without_actions;

    // 1. 过滤行内代码 `code` -> 纯文本
    let mut cleaned = String::new();
    let mut in_inline_code = false;
    for ch in text.chars() {
        if ch == '`' {
            in_inline_code = !in_inline_code;
        } else {
            cleaned.push(ch);
        }
    }
    text = cleaned;

    // 2. 清洗标题 # Header
    text = text
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim_start()
            } else if trimmed.starts_with('>') {
                trimmed.trim_start_matches('>').trim_start()
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                &trimmed[2..]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // 3. 移除粗体/斜体格式符 **bold**, *italic*, ~~strikethrough~~
    text = text.replace("**", "").replace("*", "").replace("~~", "");

    // 4. 清理多余空白
    let words: Vec<&str> = text.split_whitespace().collect();
    words.join(" ")
}

fn split_into_clean_sentences(text: &str, is_final: bool) -> Vec<String> {
    let mut chunker = SentenceChunker::new(true);
    let mut list = chunker.push_delta(text);
    if is_final {
        list.extend(chunker.finish());
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_basic_sentences() {
        let mut chunker = SentenceChunker::new(false);
        let s1 = chunker.push_delta("你好！我是Miyu。");
        assert_eq!(s1, vec!["你好！", "我是Miyu。"]);
    }

    #[test]
    fn test_chunker_streaming_accumulation() {
        let mut chunker = SentenceChunker::new(false);
        let s1 = chunker.push_delta("这是一段很长的");
        assert!(s1.is_empty());
        let s2 = chunker.push_delta("对话，请问你今天过得好吗？");
        assert_eq!(s2, vec!["这是一段很长的对话，请问你今天过得好吗？"]);
    }

    #[test]
    fn test_chunker_skips_code_block() {
        let mut chunker = SentenceChunker::new(false);
        let s1 = chunker.push_delta("下面是代码：\n```rust\nfn main() {}\n```\n解释完毕。");
        let s2 = chunker.finish();
        let mut all = s1;
        all.extend(s2);
        assert_eq!(all, vec!["下面是代码：", "解释完毕。"]);
    }

    #[test]
    fn test_markdown_cleaning() {
        let raw = "**重要提示**：请访问 [Miyu文档](https://example.com) 获取详情。";
        let clean = clean_markdown_for_speech(raw);
        assert_eq!(clean, "重要提示：请访问 [Miyu文档](https://example.com) 获取详情。");
    }

    #[test]
    fn test_strip_action_descriptions() {
        let raw = "（顺势把你往自己怀里一带，另一只手按住你的后脑勺）哈？你看看到现在几点了";
        assert_eq!(clean_markdown_for_speech(raw), "哈？你看看到现在几点了");

        let raw_asterisk = "*脸红着把头转过去* 谁允许你这么看着我了？（心虚地咳嗽）";
        assert_eq!(clean_markdown_for_speech(raw_asterisk), "谁允许你这么看着我了？");

        let raw_pure = "（默默低头不说话）";
        assert_eq!(clean_markdown_for_speech(raw_pure), "");
    }
}
