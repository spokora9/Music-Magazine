// SHED POWER: 4-Pole Zero-Delay Feedback Ladder Filter
use std::f32::consts::PI;

pub struct LadderFilter {
    pub sample_rate: f32,
    pub cutoff: f32,
    pub resonance: f32,
    // State variables for the 4 poles
    d1: f32,
    d2: f32,
    d3: f32,
    d4: f32,
}

impl LadderFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            cutoff: 1000.0,
            resonance: 0.5,
            d1: 0.0,
            d2: 0.0,
            d3: 0.0,
            d4: 0.0,
        }
    }

    pub fn process(&mut self, input_sample: f32) -> f32 {
        // Frequency coefficient
        let g = (PI * self.cutoff.min(self.sample_rate * 0.45) / self.sample_rate).tan();
        // Feedback coefficient (resonance)
        let k = self.resonance * 3.8;

        let sat = |x: f32| x.tanh();
        let clamp = |x: f32| {
            if x.is_finite() {
                x.clamp(-8.0, 8.0)
            } else {
                0.0
            }
        };

        // Zero-Delay Feedback loop approximation
        let input = clamp((input_sample - sat(self.d4 * k)) / (1.0 + g));

        self.d1 = clamp(self.d1 + g * (sat(input) - sat(self.d1)));
        self.d2 = clamp(self.d2 + g * (sat(self.d1) - sat(self.d2)));
        self.d3 = clamp(self.d3 + g * (sat(self.d2) - sat(self.d3)));
        self.d4 = clamp(self.d4 + g * (sat(self.d3) - sat(self.d4)));

        self.d4
    }
}
