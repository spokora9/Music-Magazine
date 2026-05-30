// SHED POWER: Synthesizer Voice Management
use crate::audio::{Adsr, AdsrState, LadderFilter, Oscillator};

pub struct Voice {
    pub osc1: Oscillator,
    pub osc2: Oscillator,
    pub filter: LadderFilter,
    pub env: Adsr,
    pub note: u8,
    pub active: bool,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            osc1: Oscillator::new(sample_rate),
            osc2: Oscillator::new(sample_rate),
            filter: LadderFilter::new(sample_rate),
            env: Adsr::new(sample_rate),
            note: 0,
            active: false,
        }
    }

    pub fn note_on(&mut self, note: u8, _velocity: u8) {
        self.note = note;
        self.active = true;

        let freq = 440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0);
        self.osc1.set_frequency(freq);
        self.osc2.set_frequency(freq * 1.005); // Detune

        self.env.note_on();
    }

    pub fn note_off(&mut self) {
        self.env.note_off();
    }

    pub fn silence(&mut self) {
        self.active = false;
        self.env.state = AdsrState::Idle;
        self.env.value = 0.0;
    }

    pub fn next_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let osc_mix = (self.osc1.next_sample() * 0.6) + (self.osc2.next_sample() * 0.4);
        let filtered = self.filter.process(osc_mix);
        let env_val = self.env.next_sample();

        if self.env.state == AdsrState::Idle
            || (self.env.state == AdsrState::Sustain && self.env.value <= 0.001)
        {
            self.active = false;
        }

        filtered * env_val
    }
}
