// SHED POWER: Procedural Drum Synthesis
// Ported from original JS Worklet

use std::f32::consts::PI;

pub struct Kick {
    sample_rate: f32,
    phase: f32,
    click_phase: f32,
    envelope: f32,
    active: bool,
}

impl Kick {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            phase: 0.0,
            click_phase: 0.0,
            envelope: 0.0,
            active: false,
        }
    }

    pub fn trigger(&mut self) {
        self.active = true;
        self.phase = 0.0;
        self.click_phase = 0.0;
        self.envelope = 1.0;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.envelope = 0.0;
    }

    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Frequency Envelope (Sweep 150Hz -> 50Hz)
        let freq = 50.0 + (100.0 * self.envelope);
        let dt = freq / self.sample_rate;
        self.phase = (self.phase + dt) % 1.0;

        let body = (self.phase * 2.0 * PI).sin();

        // Click (High freq burst)
        let click_freq = 1000.0;
        let click_dt = click_freq / self.sample_rate;
        self.click_phase = (self.click_phase + click_dt) % 1.0;
        let click = (self.click_phase * 2.0 * PI).sin() * self.envelope;

        // Amplitude Envelope Decay
        self.envelope *= 0.9995; // Decay
        if self.envelope < 0.001 {
            self.active = false;
        }

        (body * 0.8 + click * 0.2) * self.envelope
    }
}

pub struct Snare {
    sample_rate: f32,
    phase: f32,
    envelope: f32,
    active: bool,
    // Simple noise state
    noise_seed: u32,
}

impl Snare {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            phase: 0.0,
            envelope: 0.0,
            active: false,
            noise_seed: 12345,
        }
    }

    pub fn trigger(&mut self) {
        self.active = true;
        self.envelope = 1.0;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.envelope = 0.0;
    }

    fn noise(&mut self) -> f32 {
        self.noise_seed = self
            .noise_seed
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        (self.noise_seed as f32 / 4294967296.0) * 2.0 - 1.0
    }

    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let freq = 180.0;
        self.phase = (self.phase + freq / self.sample_rate) % 1.0;
        let body = (self.phase * 2.0 * PI).sin();

        let noise = self.noise();

        self.envelope *= 0.999;
        if self.envelope < 0.001 {
            self.active = false;
        }

        (body * 0.5 + noise * 0.5) * self.envelope
    }
}

pub struct HiHat {
    sample_rate: f32,
    phases: [f32; 6],
    ratios: [f32; 6],
    envelope: f32,
    active: bool,
}

impl HiHat {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            phases: [0.0; 6],
            ratios: [1.0, 1.4983, 1.7424, 1.9858, 2.4631, 2.6306],
            envelope: 0.0,
            active: false,
        }
    }

    pub fn trigger(&mut self) {
        self.active = true;
        self.envelope = 1.0;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.envelope = 0.0;
    }

    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let base_freq = 8000.0;
        let mut sum = 0.0;

        for i in 0..6 {
            let f = base_freq * self.ratios[i];
            let dt = f / self.sample_rate;
            self.phases[i] = (self.phases[i] + dt) % 1.0;
            // Square wave logic
            sum += if self.phases[i] < 0.5 { 1.0 } else { -1.0 };
        }

        let signal = sum / 6.0;

        self.envelope *= 0.995; // Fast decay
        if self.envelope < 0.001 {
            self.active = false;
        }

        signal * self.envelope
    }
}
