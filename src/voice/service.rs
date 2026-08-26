//! 语音服务总调度器 (VoiceService)。
//!
//! 负责协调流式断句、并发预取合成与有序音频播放，并响应毫秒级打断（Barge-in）。

use crate::voice::chunker::SentenceChunker;
use crate::voice::player::VoicePlayer;
use crate::voice::traits::VoiceEngine;
use crate::voice::types::VoiceConfig;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub struct VoiceService {
    config: VoiceConfig,
    chunker: SentenceChunker,
    player: VoicePlayer,
    engine: VoiceEngine,
    cancel_token: CancellationToken,
    sender: Option<UnboundedSender<String>>,
    worker_handle: Option<JoinHandle<()>>,
}

impl VoiceService {
    pub fn new(config: VoiceConfig) -> Self {
        let chunker = SentenceChunker::new(config.read_code_blocks);
        let player = VoicePlayer::new();
        let engine = VoiceEngine::new(config.engine);
        let cancel_token = CancellationToken::new();

        let mut service = Self {
            config,
            chunker,
            player,
            engine,
            cancel_token,
            sender: None,
            worker_handle: None,
        };

        if service.config.enabled {
            service.start_worker();
        }

        service
    }

    /// 启动后台合成与流水线调度任务。
    fn start_worker(&mut self) {
        let (tx, mut rx) = unbounded_channel::<String>();
        let cancel_token = CancellationToken::new();
        self.cancel_token = cancel_token.clone();
        self.sender = Some(tx);

        let engine = self.engine.clone();
        let config = self.config.clone();
        let player = self.player.clone();

        let handle = tokio::spawn(async move {
            while let Some(sentence) = rx.recv().await {
                if cancel_token.is_cancelled() {
                    break;
                }

                // 异步合成单句
                let engine = engine.clone();
                let config = config.clone();
                let player = player.clone();
                let token = cancel_token.clone();

                tokio::select! {
                    _ = token.cancelled() => {
                        break;
                    }
                    res = engine.synthesize(&sentence, &config) => {
                        if let Ok(audio_bytes) = res {
                            if !token.is_cancelled() && !audio_bytes.is_empty() {
                                player.play_audio(audio_bytes);
                            }
                        }
                    }
                }
            }
        });

        self.worker_handle = Some(handle);
    }

    /// 喂入 LLM 吐出的流式文本片段（Delta）。
    pub fn feed_delta(&mut self, delta: &str) {
        if !self.config.enabled {
            return;
        }

        let sentences = self.chunker.push_delta(delta);
        for sent in sentences {
            self.dispatch_sentence(sent);
        }
    }

    /// 当前回合流式生成结束，刷新剩余未成句的尾部文本。
    pub fn finish_stream(&mut self) {
        if !self.config.enabled {
            return;
        }

        let sentences = self.chunker.finish();
        for sent in sentences {
            self.dispatch_sentence(sent);
        }
    }

    fn dispatch_sentence(&self, sentence: String) {
        if let Some(tx) = &self.sender {
            let _ = tx.send(sentence);
        }
    }

    /// 毫秒级打断（Barge-in）：清空所有排队任务与播放器声音。
    pub fn interrupt(&mut self) {
        self.cancel_token.cancel();
        self.player.interrupt();
        if let Some(handle) = self.worker_handle.take() {
            handle.abort();
        }
        // 重置状态与断句缓冲
        self.chunker = SentenceChunker::new(self.config.read_code_blocks);
        if self.config.enabled {
            self.start_worker();
        }
    }

    /// 检查是否有音频正在播放。
    pub fn is_playing(&self) -> bool {
        self.player.is_playing()
    }

    /// 更新语音配置。
    pub fn update_config(&mut self, new_config: VoiceConfig) {
        let was_enabled = self.config.enabled;
        self.config = new_config;
        self.engine = VoiceEngine::new(self.config.engine);
        self.chunker = SentenceChunker::new(self.config.read_code_blocks);

        if self.config.enabled && !was_enabled {
            self.start_worker();
        } else if !self.config.enabled && was_enabled {
            self.interrupt();
        }
    }
}
