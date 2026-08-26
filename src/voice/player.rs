//! 基于 rodio 的异步音频播放器与抢占打断控制器。
//!
//! 在后台独立线程中管理音频 Sink，支持流式入队播放，并提供毫秒级打断（Barge-in）能力。
//! 若当前环境无音频输出设备（如无头服务器），自动降级为 No-op，不阻塞主流程。

use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

enum AudioCommand {
    Play(Vec<u8>),
    Interrupt,
    Shutdown,
}

#[derive(Clone)]
pub struct VoicePlayer {
    sender: Option<Sender<AudioCommand>>,
    is_playing: Arc<AtomicBool>,
}

impl VoicePlayer {
    pub fn new() -> Self {
        let (tx, rx) = channel::<AudioCommand>();
        let is_playing = Arc::new(AtomicBool::new(false));
        let playing_flag = Arc::clone(&is_playing);

        // 尝试在独立线程中初始化音频驱动
        let spawn_res = thread::Builder::new()
            .name("miyu-audio-player".to_string())
            .spawn(move || {
                run_player_thread(rx, playing_flag);
            });

        if spawn_res.is_err() {
            return Self {
                sender: None,
                is_playing,
            };
        }

        Self {
            sender: Some(tx),
            is_playing,
        }
    }

    /// 追加一段音频数据（MP3 或 WAV）排队播放。
    pub fn play_audio(&self, audio_bytes: Vec<u8>) {
        if audio_bytes.is_empty() {
            return;
        }
        if let Some(sender) = &self.sender {
            let _ = sender.send(AudioCommand::Play(audio_bytes));
        }
    }

    /// 毫秒级打断（Barge-in）：清空当前播放和待播放队列，立即静音。
    pub fn interrupt(&self) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(AudioCommand::Interrupt);
        }
        self.is_playing.store(false, Ordering::SeqCst);
    }

    /// 检查当前是否正在播放音频。
    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
    }
}

impl Drop for VoicePlayer {
    fn drop(&mut self) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(AudioCommand::Shutdown);
        }
    }
}

fn run_player_thread(rx: Receiver<AudioCommand>, is_playing: Arc<AtomicBool>) {
    // 尝试获取默认音频输出流
    let Ok((_stream, handle)) = rodio::OutputStream::try_default() else {
        // 无音频输出设备环境，排空命令防止发送端阻塞
        while let Ok(cmd) = rx.recv() {
            if matches!(cmd, AudioCommand::Shutdown) {
                break;
            }
        }
        return;
    };

    let mut current_sink = rodio::Sink::try_new(&handle).ok();

    loop {
        match rx.recv() {
            Ok(AudioCommand::Play(bytes)) => {
                if let Some(sink) = &current_sink {
                    if let Ok(source) = rodio::Decoder::new(Cursor::new(bytes)) {
                        sink.append(source);
                        is_playing.store(true, Ordering::SeqCst);
                    }
                }
            }
            Ok(AudioCommand::Interrupt) => {
                // 停止并丢弃当前 Sink，重建全新 Sink 实现即时打断
                if let Some(sink) = current_sink.take() {
                    sink.stop();
                }
                current_sink = rodio::Sink::try_new(&handle).ok();
                is_playing.store(false, Ordering::SeqCst);
            }
            Ok(AudioCommand::Shutdown) | Err(_) => {
                if let Some(sink) = current_sink.take() {
                    sink.stop();
                }
                is_playing.store(false, Ordering::SeqCst);
                break;
            }
        }

        // 定期同步 playing 状态
        if let Some(sink) = &current_sink {
            if sink.empty() {
                is_playing.store(false, Ordering::SeqCst);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_lifecycle() {
        let player = VoicePlayer::new();
        assert!(!player.is_playing());
        player.interrupt();
        assert!(!player.is_playing());
    }
}
