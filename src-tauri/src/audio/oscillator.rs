// SHED POWER: PolyBLEP Anti-Aliased Oscillator
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

pub struct Oscillator {
    pub waveform: Waveform,
    pub sample_rate: f32,
    pub phase: f32,
    pub frequency: f32,
}

impl Oscillator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            waveform: Waveform::Saw,
            sample_rate,
            phase: 0.0,
            frequency: 440.0,
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq;
    }

    pub fn next_sample(&mut self) -> f32 {
        let dt = self.frequency / self.sample_rate;
        let out = match self.waveform {
            Waveform::Sine => (self.phase * 2.0 * PI).sin(),
            Waveform::Saw => {
                let v = 2.0 * self.phase - 1.0;
                v - self.poly_blep(self.phase, dt)
            }
            Waveform::Square => {
                let v = if self.phase < 0.5 { 1.0 } else { -1.0 };
                v + self.poly_blep(self.phase, dt) - self.poly_blep((self.phase + 0.5) % 1.0, dt)
            }
            Waveform::Triangle => 2.0 * (1.0 - 2.0 * (self.phase - 0.5).abs()) - 1.0,
            Waveform::Noise => rand::random::<f32>() * 2.0 - 1.0,
        };

        self.phase = (self.phase + dt) % 1.0;
        out
    }

    // PolyBLEP helper to remove aliasing
    fn poly_blep(&self, mut t: f32, dt: f32) -> f32 {
        if t < dt {
            t /= dt;
            t + t - t * t - 1.0
        } else if t > 1.0 - dt {
            t = (t - 1.0) / dt;
            t * t + t + t + 1.0
        } else {
            0.0
        }
    }
}
