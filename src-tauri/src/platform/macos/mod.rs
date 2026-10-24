pub mod ffi;
pub mod audio;
pub mod brightness;
pub mod media;
pub mod input;
pub mod context;
pub mod dial;
pub mod now_playing;

use crate::platform::traits::*;

pub use audio::MacAudio;
pub use brightness::MacBrightness;
pub use media::MacMedia;
pub use input::MacInput;
pub use context::MacContext;
pub use dial::MacDial;

pub struct Platform {
    audio: MacAudio,
    brightness: MacBrightness,
    media: MacMedia,
    input: MacInput,
    dial: MacDial,
    context: MacContext,
}

impl Platform {
    pub fn new() -> Self {
        Self {
            audio: MacAudio::new(),
            brightness: MacBrightness::new(),
            media: MacMedia::new(),
            input: MacInput::new(),
            dial: MacDial::new(),
            context: MacContext::new(),
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
        &self.input
    }
    fn dial(&self) -> &dyn DialBackend {
        &self.dial
    }
    fn context(&self) -> &dyn ContextDetector {
        &self.context
    }
}
