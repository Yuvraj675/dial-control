pub mod audio;
pub mod brightness;
pub mod context;
pub mod dial;
pub mod input;
pub mod media;

use crate::platform::traits::*;
use audio::LinuxAudio;
use brightness::LinuxBrightness;
use context::LinuxContext;
use dial::LinuxDial;
use input::LinuxInput;
use media::LinuxMedia;
use std::sync::Arc;

pub struct Platform {
    audio: LinuxAudio,
    input: Arc<LinuxInput>,
    brightness: LinuxBrightness,
    media: LinuxMedia,
    dial: LinuxDial,
    context: LinuxContext,
}

impl Platform {
    pub fn new() -> Self {
        let input = Arc::new(LinuxInput::new());
        let audio = LinuxAudio::new();
        let brightness = LinuxBrightness::new(input.clone());
        let media = LinuxMedia::new(input.clone());
        let dial = LinuxDial::new();
        let context = LinuxContext::new();

        Self {
            audio,
            input,
            brightness,
            media,
            dial,
            context,
        }
    }
}

impl PlatformProvider for Platform {
    fn audio(&self) -> &dyn SystemAudio {
        &self.audio
    }
    fn brightness(&self) -> &dyn SystemBrightness {
        &self.brightness
    }
    fn media(&self) -> &dyn MediaControl {
        &self.media
    }
    fn input(&self) -> &dyn VirtualInput {
        self.input.as_ref()
    }
    fn dial(&self) -> &dyn DialBackend {
        &self.dial
    }
    fn context(&self) -> &dyn ContextDetector {
        &self.context
    }
}
