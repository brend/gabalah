use crate::apu::StereoSample;

pub trait AudioSink {
    fn push_samples(&mut self, samples: &[StereoSample]);
}

#[derive(Debug, Default)]
pub struct SilentAudioSink {
    pushed_samples: u64,
}

impl AudioSink for SilentAudioSink {
    fn push_samples(&mut self, samples: &[StereoSample]) {
        self.pushed_samples += samples.len() as u64;
    }
}

pub fn create_default_sink() -> Box<dyn AudioSink> {
    Box::<SilentAudioSink>::default()
}
