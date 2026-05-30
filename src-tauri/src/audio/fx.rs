// SHED POWER: Per-part looper insert effects.

use std::f32::consts::PI;

fn sanitize(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-4.0, 4.0)
    } else {
        0.0
    }
}

fn one_pole_alpha(cutoff: f32, sample_rate: f32) -> f32 {
    let cutoff = cutoff.clamp(10.0, sample_rate * 0.45);
    1.0 - (-2.0 * PI * cutoff / sample_rate).exp()
}

pub struct Delay {
    buffer: Vec<f32>,
    write_ptr: usize,
    pub time: f32,
    pub feedback: f32,
    pub mix: f32,
    sample_rate: f32,
}

impl Delay {
    pub fn new(sample_rate: f32) -> Self {
        let max_delay = (sample_rate * 2.0) as usize;
        Self {
            buffer: vec![0.0; max_delay.max(1)],
            write_ptr: 0,
            time: 0.3,
            feedback: 0.35,
            mix: 0.0,
            sample_rate,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let delay_samples = (self.time.clamp(0.005, 1.8) * self.sample_rate) as usize;
        let delay_samples = delay_samples.min(self.buffer.len() - 1).max(1);
        let read_ptr = (self.write_ptr + self.buffer.len() - delay_samples) % self.buffer.len();
        let delayed_sample = self.buffer[read_ptr];

        self.buffer[self.write_ptr] =
            sanitize(input + delayed_sample * self.feedback.clamp(0.0, 0.92));
        self.write_ptr = (self.write_ptr + 1) % self.buffer.len();

        input + delayed_sample * self.mix.clamp(0.0, 1.0)
    }
}

struct Comb {
    buffer: Vec<f32>,
    index: usize,
    filter_store: f32,
}

impl Comb {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size.max(1)],
            index: 0,
            filter_store: 0.0,
        }
    }

    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let output = self.buffer[self.index];
        self.filter_store = output * (1.0 - damp) + self.filter_store * damp;
        self.buffer[self.index] = sanitize(input + self.filter_store * feedback);
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

pub struct SimpleReverb {
    combs: Vec<Comb>,
    pub size: f32,
    damp: f32,
}

impl SimpleReverb {
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate / 44100.0;
        let sizes = [1116, 1188, 1277, 1356, 1422, 1491]
            .into_iter()
            .map(|size| (size as f32 * scale) as usize)
            .map(Comb::new)
            .collect();
        Self {
            combs: sizes,
            size: 0.55,
            damp: 0.22,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let feedback = 0.55 + self.size.clamp(0.0, 1.0) * 0.38;
        let mut wet = 0.0;
        for comb in &mut self.combs {
            wet += comb.process(input * 0.35, feedback, self.damp);
        }
        wet / self.combs.len() as f32
    }
}

pub struct Chorus {
    buffer: Vec<f32>,
    write_ptr: usize,
    phase: f32,
    sample_rate: f32,
    pub mix: f32,
    pub depth: f32,
    pub rate: f32,
}

impl Chorus {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buffer: vec![0.0; (sample_rate * 0.08) as usize],
            write_ptr: 0,
            phase: 0.0,
            sample_rate,
            mix: 0.0,
            depth: 0.55,
            rate: 0.28,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.buffer[self.write_ptr] = input;

        let base_delay = 0.018 * self.sample_rate;
        let depth_samples = self.depth.clamp(0.0, 1.0) * 0.014 * self.sample_rate;
        let lfo = (self.phase * 2.0 * PI).sin() * 0.5 + 0.5;
        let delay = base_delay + lfo * depth_samples;
        let read_pos =
            (self.write_ptr as f32 + self.buffer.len() as f32 - delay) % self.buffer.len() as f32;
        let read_a = read_pos.floor() as usize;
        let read_b = (read_a + 1) % self.buffer.len();
        let frac = read_pos - read_a as f32;
        let wet = self.buffer[read_a] * (1.0 - frac) + self.buffer[read_b] * frac;

        self.write_ptr = (self.write_ptr + 1) % self.buffer.len();
        self.phase = (self.phase + self.rate.clamp(0.02, 4.0) / self.sample_rate) % 1.0;

        input + wet * self.mix.clamp(0.0, 1.0)
    }
}

pub struct FxChain {
    pub drive: f32,
    pub gain: f32,
    pub delay: Delay,
    pub reverb: SimpleReverb,
    pub reverb_mix: f32,
    pub lowpass: f32,
    pub highpass: f32,
    pub chorus: Chorus,
    pub tremolo_depth: f32,
    pub slicer_amount: f32,
    pub bitcrush: f32,
    pub compressor: f32,
    sample_rate: f32,
    lowpass_state: f32,
    highpass_state: f32,
    tremolo_phase: f32,
    slicer_phase: f32,
    crush_counter: usize,
    crush_hold: f32,
}

impl FxChain {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            drive: 0.0,
            gain: 1.0,
            delay: Delay::new(sample_rate),
            reverb: SimpleReverb::new(sample_rate),
            reverb_mix: 0.0,
            lowpass: 1.0,
            highpass: 0.0,
            chorus: Chorus::new(sample_rate),
            tremolo_depth: 0.0,
            slicer_amount: 0.0,
            bitcrush: 0.0,
            compressor: 0.0,
            sample_rate,
            lowpass_state: 0.0,
            highpass_state: 0.0,
            tremolo_phase: 0.0,
            slicer_phase: 0.0,
            crush_counter: 0,
            crush_hold: 0.0,
        }
    }

    pub fn set_param(&mut self, id: u8, value: f32) {
        let value = value.clamp(0.0, 1.0);
        match id {
            0 => self.gain = value * 2.0,
            1 => self.drive = value,
            2 => self.lowpass = value,
            3 => self.highpass = value,
            4 => self.delay.time = 0.01 + value * 1.2,
            5 => self.delay.feedback = value * 0.92,
            6 => self.delay.mix = value,
            7 => self.reverb_mix = value,
            8 => self.reverb.size = value,
            9 => self.chorus.mix = value,
            10 => self.tremolo_depth = value,
            11 => self.slicer_amount = value,
            12 => self.bitcrush = value,
            13 => self.compressor = value,
            _ => {}
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let mut out = sanitize(input);

        if self.highpass > 0.001 {
            let cutoff = 20.0 + self.highpass * self.highpass * 1800.0;
            let alpha = one_pole_alpha(cutoff, self.sample_rate);
            self.highpass_state += alpha * (out - self.highpass_state);
            out -= self.highpass_state;
        }

        if self.lowpass < 0.999 {
            let cutoff = 160.0 + self.lowpass * self.lowpass * 18000.0;
            let alpha = one_pole_alpha(cutoff, self.sample_rate);
            self.lowpass_state += alpha * (out - self.lowpass_state);
            out = self.lowpass_state;
        }

        if self.drive > 0.001 {
            let k = self.drive * 16.0;
            out = (out * (1.0 + k)).atan() / (PI / 2.0) * 0.85;
        }

        if self.bitcrush > 0.001 {
            let hold_samples = (1.0 + self.bitcrush * 28.0) as usize;
            if self.crush_counter == 0 {
                let steps = 2.0_f32.powf(15.0 - self.bitcrush * 10.0);
                self.crush_hold = (out * steps).round() / steps;
            }
            out = self.crush_hold;
            self.crush_counter = (self.crush_counter + 1) % hold_samples.max(1);
        }

        out = self.chorus.process(out);
        out = self.delay.process(out);

        if self.reverb_mix > 0.001 {
            let wet = self.reverb.process(out);
            out = out * (1.0 - self.reverb_mix * 0.55) + wet * self.reverb_mix;
        } else {
            let _ = self.reverb.process(out * 0.05);
        }

        if self.tremolo_depth > 0.001 {
            let lfo = (self.tremolo_phase * 2.0 * PI).sin() * 0.5 + 0.5;
            let amp = 1.0 - self.tremolo_depth * lfo;
            out *= amp;
            self.tremolo_phase = (self.tremolo_phase + 5.0 / self.sample_rate) % 1.0;
        }

        if self.slicer_amount > 0.001 {
            let phase = self.slicer_phase;
            let gate = if phase < 0.45 {
                1.0
            } else if phase < 0.55 {
                1.0 - ((phase - 0.45) / 0.10) * self.slicer_amount
            } else {
                1.0 - self.slicer_amount
            };
            out *= gate;
            self.slicer_phase = (self.slicer_phase + 8.0 / self.sample_rate) % 1.0;
        }

        if self.compressor > 0.001 {
            let threshold = 0.65 - self.compressor * 0.45;
            let abs = out.abs();
            if abs > threshold {
                let over = abs - threshold;
                let compressed = threshold + over * (1.0 - self.compressor * 0.85);
                out = out.signum() * compressed;
            }
            out *= 1.0 + self.compressor * 0.35;
        }

        sanitize(out * self.gain).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::FxChain;

    #[test]
    fn fx_chain_is_bypassed_by_default() {
        let mut fx = FxChain::new(48_000.0);
        assert!((fx.process(0.25) - 0.25).abs() < 0.0001);
    }

    #[test]
    fn all_fx_params_stay_finite() {
        let mut fx = FxChain::new(48_000.0);
        for id in 0..=13 {
            fx.set_param(id, 0.8);
        }

        for _ in 0..4096 {
            let out = fx.process(0.35);
            assert!(out.is_finite());
            assert!((-1.0..=1.0).contains(&out));
        }
    }
}
