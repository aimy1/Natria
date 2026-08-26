#[cfg(test)]
mod tests {
    use crate::voice::types::{VoiceConfig, VoicePreset};
    use crate::voice::service::VoiceService;

    #[test]
    fn test_voice_presets() {
        let mut config = VoiceConfig::default();
        
        VoicePreset::NeuroClassic.apply_to(&mut config);
        assert_eq!(config.voice, "en-US-AshleyNeural");
        assert_eq!(config.pitch, "+45Hz");
        assert_eq!(config.rate, "+8%");

        VoicePreset::NeuroEvil.apply_to(&mut config);
        assert_eq!(config.voice, "en-US-SaraNeural");
        assert_eq!(config.pitch, "+15Hz");
        assert_eq!(config.rate, "-5%");

        VoicePreset::MiyuChinese.apply_to(&mut config);
        assert_eq!(config.voice, "zh-CN-XiaoxiaoNeural");
        assert_eq!(config.pitch, "+30Hz");
        assert_eq!(config.rate, "+5%");
    }

    #[tokio::test]
    async fn test_voice_service_lifecycle() {
        let config = VoiceConfig {
            enabled: false,
            ..Default::default()
        };
        let mut service = VoiceService::new(config);
        assert!(!service.is_playing());
        
        service.feed_delta("你好，这是一段测试文本。");
        service.finish_stream();
        service.interrupt();
        assert!(!service.is_playing());
    }

    #[tokio::test]
    async fn test_edge_tts_direct() {
        let config = VoiceConfig::default();
        let engine = crate::voice::engines::edge_tts::EdgeTtsEngine::new();
        let audio = engine.synthesize("你好，我是 Miyu", &config).await;
        println!("Test Edge TTS result: {:?}", audio.as_ref().map(|b| b.len()));
        assert!(audio.is_ok());
    }
}
