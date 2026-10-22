pub mod ffi;
pub mod audio;
pub mod brightness;
pub mod media;
pub mod input;
pub mod context;
pub mod dial;

use crate::platform::traits::*;

pub use audio::WindowsAudio;
pub use brightness::WindowsBrightness;
pub use media::WindowsMedia;
pub use input::WindowsInput;
pub use context::WindowsContext;
pub use dial::WindowsDial;

pub struct Platform {
    audio: WindowsAudio,
    brightness: WindowsBrightness,
    media: WindowsMedia,
    input: WindowsInput,
    dial: WindowsDial,
    context: WindowsContext,
}

impl Platform {
    pub fn new() -> Self {
        Self {
            audio: WindowsAudio::new(),
            brightness: WindowsBrightness::new(),
            media: WindowsMedia::new(),
            input: WindowsInput::new(),
            dial: WindowsDial::new(),
            context: WindowsContext::new(),
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
