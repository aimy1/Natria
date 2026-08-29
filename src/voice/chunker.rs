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

    let mut paren_depth = 0;
    let mut paren_start_idx = 0;

    for (i, &(_byte_offset, ch)) in chars.iter().enumerate() {
        if ch == '（' || ch == '(' {
            if paren_depth == 0 {
                paren_start_idx = i;
            }
            paren_depth += 1;
        } else if ch == '）' || ch == ')' {
            if paren_depth > 0 {
                paren_depth -= 1;
            }
        }

        // 如果括号距离过长（> 25 字符）仍未闭合，视为普通标点或长引用，不再阻塞切句
        let in_short_paren = paren_depth > 0 && (i.saturating_sub(paren_start_idx) < 25);
        if in_short_paren {
            continue;
        }

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

/// 检查某段括号内的文本是否为纯角色扮演动作/舞台描写（如“叹气”、“轻笑”、“捂脸”等），
/// 坚决避免误伤“例如 123”、“点击下一步”、“动态规划”等正常说明性括号内容。
fn is_theatrical_action(inner: &str) -> bool {
    let s = inner.trim();
    if s.is_empty() || s.chars().count() > 24 {
        return false;
    }

    // 包含明显说明性关键词、数字、版本、链接时，绝非动作描写
    let non_action_keywords = [
        "例如", "比如", "注意", "提示", "推荐", "可选", "默认", "参见", "参考",
        "包括", "即", "第", "共", "注：", "注:", "http", "url", "px", "%", "v0.", "v1.", "v2.", "v3."
    ];
    for kw in non_action_keywords {
        if s.contains(kw) {
            return false;
        }
    }

    // 含有阿拉伯数字说明（如 "(共 3 个)" 或 "(第 2 步)"）时保留
    if s.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }

    // 常见舞台/动作描写特征动词与词汇
    let action_cues = [
        "笑", "叹", "白眼", "翻白眼", "咳嗽", "脸红", "挠头", "摸头", "揉头", "托腮",
        "嘟嘴", "撇嘴", "扶额", "摊手", "傲娇", "抽泣", "小声", "轻声", "转头", "看向",
        "低头", "抬头", "别过头", "眨眼", "深吸一口气", "清了清嗓子", "愣了一下", "顿了顿",
        "把头转过去", "顺势", "心虚", "害羞", "委屈", "生气", "抱胸", "握拳", "耸肩"
    ];

    action_cues.iter().any(|&cue| s.contains(cue))
}

/// 移除文本中的纯动作描写（如 *（微笑着递给你杯子）*、*叹气*、（轻声细语）），
/// 保留所有正常的说明括号、书名号、标签和正文对话。
pub fn strip_action_descriptions(text: &str) -> String {
    let mut s = text.to_string();

    // 1. 剥离星号包裹的动作描写，如 *脸红*、*（轻笑）*、*叹了口气*
    // 注意：只剥离非粗体（非 **）且较短的星号动作，不误删正常列表项
    while let Some(start) = s.find('*') {
        // 如果是 ** 粗体前缀，则跳过
        if s[start..].starts_with("**") {
            break;
        }
        if let Some(end_offset) = s[start + 1..].find('*') {
            let end = start + 1 + end_offset;
            if !s[end..].starts_with("**") && end - start <= 40 {
                let inside = &s[start + 1..end];
                if is_theatrical_action(inside) || inside.starts_with('（') || inside.starts_with('(') {
                    s.replace_range(start..end + 1, "");
                    continue;
                }
            }
        }
        break;
    }

    // 2. 剥离中英文圆括号中的纯舞台/动作描写，保留正常解释说明
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '（' || ch == '(' {
            let close_char = if ch == '（' { '）' } else { ')' };
            let mut inside = String::new();
            let mut matched_close = false;

            for next_ch in chars.by_ref() {
                if next_ch == close_char {
                    matched_close = true;
                    break;
                }
                inside.push(next_ch);
                if inside.len() > 80 {
                    break;
                }
            }

            if matched_close && is_theatrical_action(&inside) {
                // 是纯舞台动作，剥离不朗读
                continue;
            } else {
                // 是正常说明括号，保留括号内文字并以逗号短停顿朗读
                result.push('，');
                result.push_str(&inside);
                if matched_close {
                    result.push('，');
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// 将文本中的 Markdown 标记清洗为适合语音朗读的自然纯文本。
pub fn clean_markdown_for_speech(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }

    let mut text = raw.to_string();

    // 1. 去除代码块及内容、图片与超链接
    // 1.1 图片：![alt](url) -> 移除
    while let Some(start) = text.find("![") {
        if let Some(mid) = text[start..].find("](") {
            if let Some(end) = text[start + mid..].find(')') {
                text.replace_range(start..start + mid + end + 1, "");
                continue;
            }
        }
        break;
    }

    // 1.2 Markdown 链接：[标题](url) -> 转换为 "标题"（朗读标题，忽略 URL）
    while let Some(start) = text.find('[') {
        if let Some(mid) = text[start..].find("](") {
            if let Some(end) = text[start + mid..].find(')') {
                let link_title = text[start + 1..start + mid].to_string();
                text.replace_range(start..start + mid + end + 1, &link_title);
                continue;
            }
        }
        break;
    }

    // 1.3 移除独立 URL (http:// 或 https://)
    while let Some(start) = text.find("http://").or_else(|| text.find("https://")) {
        let end = text[start..]
            .find(|c: char| c.is_whitespace() || matches!(c, '）' | ')' | ']' | '】' | '，' | '。' | '；'))
            .map(|offset| start + offset)
            .unwrap_or(text.len());
        text.replace_range(start..end, "");
    }

    // 2. 剥离角色扮演的纯舞台动作描写
    text = strip_action_descriptions(&text);

    // 3. 行内代码 `code` -> 纯文字 code
    let mut without_backticks = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch != '`' {
            without_backticks.push(ch);
        }
    }
    text = without_backticks;

    // 4. 清洗标题 # Header、引用 > Quote、列表 - / * / 1.
    let mut cleaned_lines = Vec::new();
    for line in text.lines() {
        let mut trimmed = line.trim();
        // 标题
        if trimmed.starts_with('#') {
            trimmed = trimmed.trim_start_matches('#').trim();
        }
        // 引用
        if trimmed.starts_with('>') {
            trimmed = trimmed.trim_start_matches('>').trim();
        }
        // 列表符号
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            trimmed = &trimmed[2..].trim();
        } else if let Some(dot_idx) = trimmed.find(". ") {
            if trimmed[..dot_idx].chars().all(|c| c.is_ascii_digit()) {
                trimmed = &trimmed[dot_idx + 2..].trim();
            }
        }
        if !trimmed.is_empty() {
            cleaned_lines.push(trimmed);
        }
    }
    text = cleaned_lines.join(" ");

    // 5. 移除粗体/斜体/删除线标记符 (**bold**, *italic*, ~~strikethrough~~)
    text = text.replace("***", "").replace("**", "").replace('*', "").replace("~~", "").replace("___", "").replace("__", "").replace('_', "");

    // 6. 将书名号《》、标签号【】、括号() 等转为自然语流停顿
    let mut smoothed = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '《' | '》' | '【' | '】' | '［' | '］' | '[' | ']' | '〔' | '〕' | '〈' | '〉' => {
                smoothed.push(' ');
            }
            '（' | '(' | '）' | ')' => {
                smoothed.push('，');
            }
            _ => smoothed.push(ch),
        }
    }
    text = smoothed;

    // 7. 技术名词口语化归一化
    text = text
        .replace("GPT-SoVITS", "GPT 声音模型")
        .replace("gpt-sovits", "GPT 声音模型")
        .replace("Edge-TTS", "Edge TTS")
        .replace("edge-tts", "Edge TTS")
        .replace("WebUI", "Web UI")
        .replace("webui", "Web UI")
        .replace("natria", "小盐")
        .replace("Natria", "小盐");

    // 8. 规整多重标点与空白
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut joined = words.join(" ");

    // 去除标点前的多余空格与重复或冲突标点（如 " ，" -> "，", "，。" -> "。"）
    for punc in ['，', '。', '！', '？', '；', '：', '、', ',', '.', '!', '?', ';', ':'] {
        let space_punc = format!(" {}", punc);
        let punc_str = punc.to_string();
        while joined.contains(&space_punc) {
            joined = joined.replace(&space_punc, &punc_str);
        }
    }
    while joined.contains("，。") || joined.contains("，！") || joined.contains("，？") {
        joined = joined.replace("，。", "。").replace("，！", "！").replace("，？", "？");
    }
    while joined.contains("。，") || joined.contains("！，") || joined.contains("？，") {
        joined = joined.replace("。，", "。").replace("！，", "！").replace("？，", "？");
    }

    let cleaned = joined
        .trim_start_matches(|c| matches!(c, '，' | ',' | '、' | '；' | ';' | ' ' | '\t'))
        .trim_end_matches(|c| matches!(c, '，' | ',' | '、' | '；' | ';' | ' ' | '\t'));

    cleaned.to_string()
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
        let s1 = chunker.push_delta("你好！我是小盐。");
        assert_eq!(s1, vec!["你好！", "我是小盐。"]);
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
        assert_eq!(clean, "重要提示：请访问 Miyu文档 获取详情。");

        let raw_book = "我推荐你阅读《三体》和《人工智能导论》（包含 3 个核心章节）。";
        let clean_book = clean_markdown_for_speech(raw_book);
        assert!(clean_book.contains("三体"));
        assert!(clean_book.contains("人工智能导论"));
        assert!(clean_book.contains("包含 3 个核心章节"));
    }

    #[test]
    fn test_strip_action_descriptions() {
        let raw = "（顺势把你往自己怀里一带，另一只手按住你的后脑勺）哈？你看看到现在几点了";
        assert_eq!(clean_markdown_for_speech(raw), "哈？你看看到现在几点了");

        let raw_asterisk = "*脸红着把头转过去* 谁允许你这么看着我了？（心虚地咳嗽）";
        assert_eq!(clean_markdown_for_speech(raw_asterisk), "谁允许你这么看着我了？");

        let raw_pure = "（默默低头不说话）";
        assert_eq!(clean_markdown_for_speech(raw_pure), "");

        let raw_bold_code = "**`获取`** 或 **`安装`**";
        assert_eq!(clean_markdown_for_speech(raw_bold_code), "获取 或 安装");

        let raw_explain = "请点击确认（需要管理员权限）。";
        let clean_explain = clean_markdown_for_speech(raw_explain);
        assert!(clean_explain.contains("需要管理员权限"));
    }
}

