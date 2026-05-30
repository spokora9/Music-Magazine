use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct SamplerPadParams {
    pub start: f32,
    pub end: f32,
    pub volume: f32,
    pub pitch: f32,
}

impl SamplerPadParams {
    pub fn new() -> Self {
        Self {
            start: 0.0,
            end: 1.0,
            volume: 1.0,
            pitch: 0.0,
        }
    }
}

pub struct SamplerVoice {
    pub active: bool,
    pub position: f32,
    pub sample_index: usize,
}

impl SamplerVoice {
    pub fn new() -> Self {
        Self {
            active: false,
            position: 0.0,
            sample_index: 0,
        }
    }
}

pub struct Sampler {
    pub samples: [Option<Arc<Vec<f32>>>; 16],
    pub params: [SamplerPadParams; 16],
    pub voices: [SamplerVoice; 16], // One voice per pad for simplicity (monophonic per pad)
    _sample_rate: f32,
}

impl Sampler {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            samples: Default::default(),
            params: [SamplerPadParams::new(); 16],
            voices: [
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
                SamplerVoice::new(),
            ],
            _sample_rate: sample_rate,
        }
    }

    pub fn load(&mut self, pad_id: usize, data: Vec<f32>) {
        if pad_id < 16 {
            self.samples[pad_id] = Some(Arc::new(data));
            self.params[pad_id] = SamplerPadParams::new();
        }
    }

    pub fn clear_all(&mut self) {
        self.samples = Default::default();
        self.params = [SamplerPadParams::new(); 16];
        self.stop_all();
    }

    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
            voice.position = 0.0;
        }
    }

    pub fn set_params(&mut self, pad_id: usize, start: f32, end: f32, volume: f32, pitch: f32) {
        if pad_id >= 16 {
            return;
        }

        let start = start.clamp(0.0, 0.99);
        let end = end.clamp(start + 0.01, 1.0);
        self.params[pad_id] = SamplerPadParams {
            start,
            end,
            volume: volume.clamp(0.0, 2.0),
            pitch: pitch.clamp(-24.0, 24.0),
        };
    }

    pub fn trigger(&mut self, pad_id: usize) -> bool {
        if pad_id < 16 && self.samples[pad_id].is_some() {
            self.voices[pad_id].active = true;
            self.voices[pad_id].position = self.start_sample(pad_id);
            self.voices[pad_id].sample_index = pad_id;
            return true;
        }
        false
    }

    fn start_sample(&self, pad_id: usize) -> f32 {
        self.samples[pad_id]
            .as_ref()
            .map(|sample| sample.len() as f32 * self.params[pad_id].start)
            .unwrap_or(0.0)
    }

    pub fn process(&mut self) -> f32 {
        let mut sum = 0.0;
        for voice in &mut self.voices {
            if voice.active {
                if let Some(sample) = &self.samples[voice.sample_index] {
                    let params = self.params[voice.sample_index];
                    let start = (sample.len() as f32 * params.start) as usize;
                    let end = ((sample.len() as f32 * params.end) as usize)
                        .clamp(start + 1, sample.len());
                    let idx = voice.position.floor() as usize;

                    if idx < end {
                        let next_idx = (idx + 1).min(end.saturating_sub(1));
                        let frac = voice.position - idx as f32;
                        let sample_value = sample[idx] + ((sample[next_idx] - sample[idx]) * frac);
                        let rate = 2.0_f32.powf(params.pitch / 12.0);
                        sum += sample_value * params.volume;
                        voice.position += rate;
                    } else {
                        voice.active = false;
                    }
                } else {
                    voice.active = false;
                }
            }
        }
        sum
    }
}
