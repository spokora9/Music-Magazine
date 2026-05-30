// SHED POWER: ADSR Envelope Generator

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdsrState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct Adsr {
    pub sample_rate: f32,
    pub state: AdsrState,
    pub value: f32,

    // Time in seconds
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            state: AdsrState::Idle,
            value: 0.0,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
        }
    }

    pub fn note_on(&mut self) {
        self.state = AdsrState::Attack;
    }

    pub fn note_off(&mut self) {
        self.state = AdsrState::Release;
    }

    pub fn next_sample(&mut self) -> f32 {
        let dt = 1.0 / self.sample_rate;
        let attack = self.attack.max(0.0005);
        let decay = self.decay.max(0.0005);
        let release = self.release.max(0.0005);

        match self.state {
            AdsrState::Idle => {
                self.value = 0.0;
            }
            AdsrState::Attack => {
                self.value += dt / attack;
                if self.value >= 1.0 {
                    self.value = 1.0;
                    self.state = AdsrState::Decay;
                }
            }
            AdsrState::Decay => {
                self.value -= dt / decay * (1.0 - self.sustain);
                if self.value <= self.sustain {
                    self.value = self.sustain;
                    self.state = AdsrState::Sustain;
                }
            }
            AdsrState::Sustain => {
                self.value = self.sustain;
            }
            AdsrState::Release => {
                self.value -= dt / release * self.sustain.max(0.001);
                if self.value <= 0.001 {
                    self.value = 0.0;
                    self.state = AdsrState::Idle;
                }
            }
        }
        if !self.value.is_finite() {
            self.value = 0.0;
            self.state = AdsrState::Idle;
        }
        self.value
    }
}
