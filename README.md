<p align="center">
  <img src="pics/natria-logo.png" alt="Natria Logo" width="180">
</p>

# Natria

<p align="center">
  <strong>现代化智能 AI 编程伴侣 & 伴生智能系统 (Intelligent Pair-Programming & AI Companion)</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust"></a>
  <a href="https://github.com/aimy1/Natria"><img src="https://img.shields.io/badge/repo-Natria-purple.svg" alt="Repository"></a>
</p>

---

## 🌟 关于 Natria

**Natria** 是一个由 Rust 驱动的高性能、现代化、具有丰富情感表现与强大编码能力的 AI 伴生智能系统。它不仅拥有极致顺畅的终端交互与精美的 WebUI 界面，还内置了鲜活生动的**默认核心人格——「小盐」**。

---

## ✨ 核心特性

### 1. 🎭 灵魂人设 · 「傲娇小盐」
- **生动反差**：超级傲娇、毒舌嘴硬、爱吐槽、偶尔撒娇，但行动上却细致入微、时刻关心用户；
- **智能对话**：告别刻板机械的助手腔调，像真实伙伴一样自然聊天的同时，精准解决各类技术疑难与日常问题；
- **防失忆与记忆网络**：内置多层联想记忆系统（短期日记、长期提炼、知识图谱），越聊越懂你。

### 2. 🖥️ 现代化 WebUI 界面
- **双模体验**：自适应明暗主题、Material Design 3 配色方案与沉浸式卡片栅格排版；
- **独立控制台**：全新设计的独立 **「🎙️ 语音」**、**「模型池」** 与 **「供应商管理」** 面板；
- **流畅渲染**：全功能 Markdown 解析、KaTeX 数学公式渲染、代码高亮与交互式工件（Artifacts）预览；
- **语音沉浸**：支持多引擎语音播报、音色库 CRUD 与本地音频文件即时管理，输入即打断。

### 3. ⌨️ 终端 & Shell 深度集成
- **全键盘 REPL / TUI**：支持终端快捷键浏览历史、多行编辑与可视化配置；
- **Shell 无缝唤醒**：原生集成 `fish`、`zsh`、`bash`，在终端敲击指令随叫随到。

### 4. 🛠️ 强大多模型与工具矩阵
- **多模型支持**：支持 OpenAI、Claude、Gemini、DeepSeek、Ollama、Local llama.cpp 等主流与自建 API；
- **智能工具链**：文件读写编辑、Shell 命令执行、网络实时检索（Tavily、Firecrawl、SearXNG 等）、天气查询、科学计算与图像分析。

---

## 🚀 快速开始

### 源码编译安装

确保本机已安装 [Rust 编译环境](https://rustup.rs/) (Rust 1.85+)：

```bash
# 克隆仓库
git clone https://github.com/aimy1/Natria.git
cd Natria

# 编译并运行 WebUI 模式
cargo run --release --bin natria -- web --port 8300
```

打开浏览器访问：👉 **`http://127.0.0.1:8300`**

### 命令行常用指令

```bash
# 启动 WebUI 伴随服务
natria web --port 8300

# 交互式终端 REPL 模式
natria normal

# 打开终端可视化配置面板 (TUI)
natria config

# 检查当前服务守护进程状态
natria daemon status
```

---

## ⚙️ 配置文件说明

全局配置文件位于 `~/.natria/config/config.jsonc`（Windows 下位于 `%USERPROFILE%\.natria\config\config.jsonc`，同时向下兼容 `~/.miyu`）：

- **`active_provider`**：设置当前活跃的模型供应商（如 `antigravity`、`openai`、`deepseek`、`anthropic` 等）；
- **`providers`**：配置各供应商的 `base_url`、`api_key` 和模型列表；
- **`prompt`**：管理自定义提示词人格与用户身份设定。

---

## 🤝 致谢与生态

感谢以下开源项目为 Natria 带来的灵感与技术支持：
- [Opencode](https://github.com/anomalyco/opencode)
- [Claude Code](https://github.com/anthropics/claude-code)
- [Deepseek-Harness](https://github.com/deepseek-ai/deepseek-harness)
- [AstrBot](https://github.com/AstrBotDevs/AstrBot)

---

## 📄 开源许可证

本项目基于 [MIT License](LICENSE) 开源协议发布。
