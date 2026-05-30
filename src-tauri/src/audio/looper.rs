// SHED POWER: Sample-Accurate Multi-Track Looper

pub const LOOPER_PART_COUNT: usize = 5;
pub const LOOPER_PART_NAMES: [&str; LOOPER_PART_COUNT] = ["A", "B", "C", "D", "E"];
const MIN_CAPTURED_MATERIAL_PEAK: f32 = 0.00001;

pub struct LoopPart {
    pub buffer: Vec<f32>, // Mixed cache of all committed layers.
    pub layers: Vec<Vec<f32>>,
    recording_layer: Option<Vec<f32>>,
    pub duration_samples: usize,
    pub play_ptr: usize,
    record_ptr: usize,
    pub is_recording: bool,
    pub is_playing: bool,
    pub is_muted: bool,
}

impl LoopPart {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            layers: Vec::new(),
            recording_layer: None,
            duration_samples: 0,
            play_ptr: 0,
            record_ptr: 0,
            is_recording: false,
            is_playing: false,
            is_muted: false,
        }
    }

    pub fn has_material(&self) -> bool {
        !self.layers.is_empty() && !self.buffer.is_empty()
    }

    fn has_captured_material(layer: &[f32]) -> bool {
        layer
            .iter()
            .any(|sample| sample.is_finite() && sample.abs() > MIN_CAPTURED_MATERIAL_PEAK)
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.layers.clear();
        self.recording_layer = None;
        self.duration_samples = 0;
        self.play_ptr = 0;
        self.record_ptr = 0;
        self.is_recording = false;
        self.is_playing = false;
        self.is_muted = false;
    }

    pub fn load_layers(&mut self, layers: Vec<Vec<f32>>) {
        self.clear();
        self.layers = layers
            .into_iter()
            .filter(|layer| !layer.is_empty())
            .collect();
        self.duration_samples = self.layers.iter().map(Vec::len).max().unwrap_or(0);
        self.rebuild_mix();
    }

    pub fn load_mixed_buffer(&mut self, data: Vec<f32>) {
        if data.is_empty() {
            self.clear();
        } else {
            self.load_layers(vec![data]);
        }
    }

    pub fn undo(&mut self) -> bool {
        if self.recording_layer.take().is_some() {
            self.is_recording = false;
            self.record_ptr = 0;
            println!("LOOPER: Recording layer cancelled");
            return true;
        }

        if self.layers.pop().is_some() {
            println!(
                "LOOPER: Removed last layer, {} layer(s) remain",
                self.layers.len()
            );
            self.rebuild_mix();
            if self.layers.is_empty() {
                self.clear();
            }
            true
        } else {
            false
        }
    }

    pub fn start_recording(&mut self, master_duration: usize) {
        if self.is_recording {
            return;
        }

        let loop_len = if self.duration_samples > 0 {
            self.duration_samples
        } else {
            master_duration
        };

        if loop_len > 0 {
            self.recording_layer = Some(vec![0.0; loop_len]);
            self.duration_samples = loop_len;
            self.record_ptr = if self.has_material() && !self.buffer.is_empty() {
                self.play_ptr % loop_len
            } else {
                0
            };
        } else {
            self.recording_layer = Some(Vec::new());
            self.record_ptr = 0;
        }

        self.is_recording = true;
        if self.has_material() {
            self.is_playing = true;
        }
    }

    pub fn record_sample(&mut self, input_sample: f32) {
        if let Some(layer) = &mut self.recording_layer {
            if self.duration_samples == 0 {
                layer.push(input_sample);
                self.record_ptr = layer.len();
            } else if !layer.is_empty() {
                let idx = self.record_ptr % layer.len();
                layer[idx] += input_sample;
                self.record_ptr = (idx + 1) % layer.len();
            }
        }
    }

    pub fn stop_recording(&mut self) -> bool {
        if !self.is_recording {
            return false;
        }

        self.is_recording = false;
        self.record_ptr = 0;

        let Some(mut layer) = self.recording_layer.take() else {
            self.is_playing = self.has_material();
            return false;
        };

        if layer.is_empty() {
            self.is_playing = self.has_material();
            return false;
        }

        Self::finalize_layer(&mut layer);

        if !Self::has_captured_material(&layer) {
            if !self.has_material() {
                self.duration_samples = 0;
                self.play_ptr = 0;
            }
            self.is_playing = self.has_material();
            return false;
        }

        if self.duration_samples == 0 {
            self.duration_samples = layer.len();
        }

        self.layers.push(layer);
        self.rebuild_mix();
        self.is_playing = true;
        self.play_ptr = 0;
        true
    }

    pub fn next_playback_sample(&mut self) -> Option<f32> {
        if !self.is_playing || !self.has_material() {
            return None;
        }

        let loop_len = if self.duration_samples > 0 {
            self.duration_samples
        } else {
            self.buffer.len()
        };

        if loop_len == 0 || self.buffer.is_empty() {
            self.play_ptr = 0;
            return None;
        }

        let sample = self.buffer.get(self.play_ptr).copied().unwrap_or(0.0);
        self.play_ptr += 1;
        if self.play_ptr >= loop_len {
            self.play_ptr = 0;
        }
        Some(if self.is_muted { 0.0 } else { sample })
    }

    pub fn rebuild_mix(&mut self) {
        let len = self.layers.iter().map(Vec::len).max().unwrap_or(0);
        if len == 0 {
            self.buffer.clear();
            self.duration_samples = 0;
            self.play_ptr = 0;
            return;
        }

        self.buffer = vec![0.0; len];
        for layer in &self.layers {
            for (idx, sample) in layer.iter().enumerate().take(len) {
                self.buffer[idx] += *sample;
            }
        }

        self.duration_samples = len;
        if self.play_ptr >= len {
            self.play_ptr = 0;
        }
    }

    fn finalize_layer(layer: &mut [f32]) {
        if layer.is_empty() {
            return;
        }

        let fade_len = 500.min(layer.len() / 2);
        let len = layer.len();
        for i in 0..fade_len {
            let fade = i as f32 / fade_len as f32;
            layer[i] *= fade;
            layer[len - 1 - i] *= fade;
        }

        for sample in layer.iter_mut() {
            if !sample.is_finite() {
                *sample = 0.0;
            } else {
                *sample = sample.clamp(-1.0, 1.0);
            }
        }
    }

    pub fn get_waveform(&self, points: usize) -> Vec<f32> {
        if self.buffer.is_empty() || points == 0 {
            return Vec::new();
        }
        let chunk_size = self.buffer.len().div_ceil(points).max(1);
        let mut waveform = Vec::with_capacity(points);
        for i in 0..points {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(self.buffer.len());
            if start >= self.buffer.len() {
                waveform.push(0.0);
                continue;
            }
            let mut max_val = 0.0f32;
            for j in start..end {
                let abs = self.buffer[j].abs();
                if abs > max_val {
                    max_val = abs;
                }
            }
            waveform.push(max_val);
        }
        waveform
    }
}

pub struct Looper {
    pub parts: [LoopPart; LOOPER_PART_COUNT],
    pub master_duration: usize,
    pub part_gains: [f32; LOOPER_PART_COUNT], // Per-part volume gains [A, B, C, D, E]
    // Song sequence state
    pub sequence: Vec<usize>, // Part IDs in play order
    pub sequence_active: bool,
    pub sequence_step: usize,           // Current step index
    pub sequence_sample_counter: usize, // Samples elapsed in current step
}

impl Looper {
    pub fn new() -> Self {
        Self {
            parts: std::array::from_fn(|_| LoopPart::new()),
            master_duration: 0,
            part_gains: [1.0; LOOPER_PART_COUNT],
            sequence: Vec::new(),
            sequence_active: false,
            sequence_step: 0,
            sequence_sample_counter: 0,
        }
    }

    /// Returns per-part outputs, enabling per-part FX chains.
    pub fn process(&mut self, input_samples: [f32; LOOPER_PART_COUNT]) -> [f32; LOOPER_PART_COUNT] {
        let mut per_part = [0.0f32; LOOPER_PART_COUNT];

        for (i, part) in self.parts.iter_mut().enumerate() {
            if part.is_recording {
                part.record_sample(input_samples[i]);
            }

            if let Some(sample) = part.next_playback_sample() {
                per_part[i] = sample * self.part_gains[i];
            }
        }

        // Song sequence auto-advance
        if self.sequence_active && self.master_duration > 0 {
            self.sequence_sample_counter += 1;
            if self.sequence_sample_counter >= self.master_duration {
                self.sequence_sample_counter = 0;
                self.sequence_step += 1;
                if self.sequence_step >= self.sequence.len() {
                    self.sequence_step = 0; // Loop continuously
                }
                self.activate_sequence_step();
            }
        }

        per_part
    }

    /// Check if any part is currently playing
    pub fn is_any_playing(&self) -> bool {
        self.parts.iter().any(|p| p.is_playing)
    }

    /// Check if any part is currently recording
    pub fn is_any_recording(&self) -> bool {
        self.parts.iter().any(|p| p.is_recording)
    }

    pub fn part_state(&self, part_id: usize) -> &'static str {
        let Some(part) = self.parts.get(part_id) else {
            return "empty";
        };

        if part.is_recording {
            if part.has_material() {
                "overdubbing"
            } else {
                "recording"
            }
        } else if !part.has_material() {
            "empty"
        } else if part.is_playing && part.is_muted {
            "paused"
        } else if part.is_playing {
            "playing"
        } else {
            "recorded"
        }
    }

    pub fn recalculate_master_duration(&mut self) {
        self.master_duration = self
            .parts
            .iter()
            .filter(|part| part.has_material())
            .map(|part| part.duration_samples)
            .max()
            .unwrap_or(0);
    }

    fn activate_sequence_step(&mut self) {
        for part in &mut self.parts {
            part.is_playing = false;
            part.is_recording = false;
            part.play_ptr = 0;
        }

        if let Some(&part_id) = self.sequence.get(self.sequence_step) {
            if part_id < LOOPER_PART_COUNT && self.parts[part_id].has_material() {
                self.parts[part_id].is_playing = true;
                self.parts[part_id].play_ptr = 0;
            }
        }
    }

    /// Start song sequence playback
    pub fn start_sequence(&mut self, parts: Vec<usize>) -> bool {
        if parts.is_empty() || self.master_duration == 0 {
            return false;
        }
        let parts: Vec<usize> = parts
            .into_iter()
            .filter(|&part_id| part_id < LOOPER_PART_COUNT && self.parts[part_id].has_material())
            .collect();
        if parts.is_empty() {
            return false;
        }

        // Stop all parts first
        for part in &mut self.parts {
            part.is_playing = false;
            part.is_recording = false;
            part.play_ptr = 0;
        }
        self.sequence = parts;
        self.sequence_active = true;
        self.sequence_step = 0;
        self.sequence_sample_counter = 0;
        self.activate_sequence_step();
        println!(
            "LOOPER: Sequence started with {} steps",
            self.sequence.len()
        );
        true
    }

    /// Stop song sequence
    pub fn stop_sequence(&mut self) {
        self.sequence_active = false;
        self.sequence_step = 0;
        self.sequence_sample_counter = 0;
        for part in &mut self.parts {
            part.is_playing = false;
            part.play_ptr = 0;
        }
        println!("LOOPER: Sequence stopped");
    }

    /// Get current sequence step part_id (for UI highlighting)
    pub fn current_sequence_part(&self) -> Option<usize> {
        if self.sequence_active && !self.sequence.is_empty() {
            Some(self.sequence[self.sequence_step])
        } else {
            None
        }
    }

    pub fn start_recording(&mut self, part_id: usize) {
        self.sequence_active = false;
        if let Some(part) = self.parts.get_mut(part_id) {
            part.is_muted = false;
            part.start_recording(self.master_duration);
            println!(
                "LOOPER: Started recording part {}, layers: {}, mixed len: {}",
                part_id,
                part.layer_count(),
                part.buffer.len()
            );
        }
    }

    pub fn start_overdub(&mut self, part_id: usize) {
        self.start_recording(part_id);
    }

    #[cfg(test)]
    pub fn play_part(&mut self, part_id: usize) -> bool {
        self.sequence_active = false;
        for part in &mut self.parts {
            part.is_playing = false;
            part.is_recording = false;
            part.play_ptr = 0;
        }

        if let Some(part) = self.parts.get_mut(part_id) {
            if part.has_material() {
                part.is_muted = false;
                part.is_playing = true;
                println!(
                    "LOOPER: Started playing active part {}, layers: {}, mixed len: {}",
                    part_id,
                    part.layer_count(),
                    part.buffer.len()
                );
                return true;
            }
        }

        false
    }

    fn aligned_play_ptr_for_part(&self, part_id: usize) -> usize {
        let Some(target) = self.parts.get(part_id) else {
            return 0;
        };
        let target_len = target.duration_samples.max(target.buffer.len());
        if target_len == 0 {
            return 0;
        }

        self.parts
            .iter()
            .enumerate()
            .find(|(idx, part)| *idx != part_id && part.is_playing && part.has_material())
            .map(|(_, part)| part.play_ptr % target_len)
            .unwrap_or(target.play_ptr % target_len)
    }

    pub fn toggle_part_active(&mut self, part_id: usize) -> Option<bool> {
        if part_id >= self.parts.len() || !self.parts[part_id].has_material() {
            return None;
        }

        self.sequence_active = false;
        let should_activate = self.parts[part_id].is_muted || !self.parts[part_id].is_playing;
        let aligned_play_ptr = self.aligned_play_ptr_for_part(part_id);
        let part = &mut self.parts[part_id];

        if should_activate {
            part.is_muted = false;
            part.is_playing = true;
            part.play_ptr = aligned_play_ptr;
        } else {
            part.is_muted = true;
            part.is_playing = true;
        }

        Some(!part.is_muted)
    }

    pub fn play_all_recorded(&mut self) -> bool {
        self.sequence_active = false;
        let mut any_playing = false;

        for (part_id, part) in self.parts.iter_mut().enumerate() {
            part.is_recording = false;
            if part.has_material() {
                part.is_muted = false;
                part.is_playing = true;
                part.play_ptr = 0;
                any_playing = true;
                println!(
                    "LOOPER: Started playing part {}, layers: {}, mixed len: {}",
                    part_id,
                    part.layer_count(),
                    part.buffer.len()
                );
            } else {
                part.is_muted = false;
                part.is_playing = false;
                part.play_ptr = 0;
            }
        }

        any_playing
    }

    pub fn stop(&mut self) {
        self.sequence_active = false; // Also stop sequence
        for part in &mut self.parts {
            part.is_playing = false;
            part.is_recording = false;
            part.play_ptr = 0;
        }
    }

    pub fn stop_recording(&mut self, part_id: usize) -> bool {
        let committed;
        let duration_samples;
        let layer_count;
        let buffer_len;
        {
            let Some(part) = self.parts.get_mut(part_id) else {
                return false;
            };

            if !part.is_recording {
                return false;
            }

            committed = part.stop_recording();
            duration_samples = part.duration_samples;
            layer_count = part.layer_count();
            buffer_len = part.buffer.len();
        }

        if committed {
            if self.master_duration == 0 {
                self.master_duration = duration_samples;
                println!(
                    "LOOPER: Set master duration to {} samples",
                    self.master_duration
                );
            } else {
                self.recalculate_master_duration();
            }
        }

        println!(
            "LOOPER: Stopped recording part {}, layers: {}, mixed len: {}",
            part_id, layer_count, buffer_len
        );
        committed
    }

    pub fn undo_part(&mut self, part_id: usize) -> bool {
        let changed = {
            let Some(part) = self.parts.get_mut(part_id) else {
                return false;
            };
            part.undo()
        };
        if changed {
            if self.sequence_active {
                self.stop_sequence();
            }
            self.recalculate_master_duration();
        }
        changed
    }

    pub fn clear_part(&mut self, part_id: usize) {
        if let Some(part) = self.parts.get_mut(part_id) {
            part.clear();
            if self.sequence_active {
                self.stop_sequence();
            }
            self.recalculate_master_duration();
        }
    }

    pub fn load_part_layers(&mut self, part_id: usize, layers: Vec<Vec<f32>>) {
        if let Some(part) = self.parts.get_mut(part_id) {
            part.load_layers(layers);
            self.recalculate_master_duration();
        }
    }

    pub fn load_part_buffer(&mut self, part_id: usize, data: Vec<f32>) {
        if let Some(part) = self.parts.get_mut(part_id) {
            part.load_mixed_buffer(data);
            self.recalculate_master_duration();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_five_parts() {
        let looper = Looper::new();

        assert_eq!(LOOPER_PART_COUNT, 5);
        assert_eq!(looper.parts.len(), LOOPER_PART_COUNT);
        assert_eq!(looper.part_gains.len(), LOOPER_PART_COUNT);
    }

    #[test]
    fn load_layers_rebuilds_mixed_buffer_for_playback_and_waveform() {
        let mut looper = Looper::new();

        looper.load_part_layers(0, vec![vec![0.1; 8], vec![0.2; 8]]);

        assert_eq!(looper.master_duration, 8);
        assert_eq!(looper.parts[0].layer_count(), 2);
        assert_eq!(looper.parts[0].buffer, vec![0.3; 8]);

        let waveform = looper.parts[0].get_waveform(4);
        assert_eq!(waveform, vec![0.3; 4]);
    }

    #[test]
    fn undo_removes_committed_layers_one_at_a_time() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8], vec![0.2; 8], vec![0.3; 8]]);

        assert!(looper.undo_part(0));
        assert_eq!(looper.master_duration, 8);
        assert_eq!(looper.parts[0].layer_count(), 2);
        assert_eq!(looper.parts[0].buffer, vec![0.3; 8]);

        assert!(looper.undo_part(0));
        assert_eq!(looper.parts[0].layer_count(), 1);
        assert_eq!(looper.parts[0].buffer, vec![0.1; 8]);

        assert!(looper.undo_part(0));
        assert_eq!(looper.master_duration, 0);
        assert!(!looper.parts[0].has_material());
        assert!(!looper.undo_part(0));
    }

    #[test]
    fn undo_cancels_active_recording_layer_before_committed_layers() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);
        looper.start_overdub(0);
        looper.process([0.2; LOOPER_PART_COUNT]);

        assert!(looper.parts[0].is_recording);
        assert!(looper.undo_part(0));

        assert!(!looper.parts[0].is_recording);
        assert_eq!(looper.parts[0].layer_count(), 1);
        assert_eq!(looper.parts[0].buffer, vec![0.1; 8]);
    }

    #[test]
    fn empty_part_records_to_existing_master_duration() {
        let mut looper = Looper::new();
        looper.start_recording(0);
        for _ in 0..8 {
            looper.process([0.001; LOOPER_PART_COUNT]);
        }
        assert!(looper.stop_recording(0));
        assert_eq!(looper.master_duration, 8);

        looper.start_recording(1);
        let short_take_samples = 3;
        for _ in 0..short_take_samples {
            looper.process([0.002; LOOPER_PART_COUNT]);
        }
        assert!(looper.stop_recording(1));

        assert_eq!(looper.master_duration, 8);
        assert_eq!(looper.parts[1].layer_count(), 1);
        assert_eq!(looper.parts[1].buffer.len(), 8);
    }

    #[test]
    fn recording_uses_per_part_input_sample() {
        let mut looper = Looper::new();

        looper.start_recording(0);
        for _ in 0..2000 {
            looper.process([0.1, 0.2, 0.3, 0.4, 0.5]);
        }
        assert!(looper.stop_recording(0));

        looper.start_recording(1);
        for _ in 0..2000 {
            looper.process([0.4, 0.5, 0.6, 0.7, 0.8]);
        }
        assert!(looper.stop_recording(1));

        looper.start_recording(4);
        for _ in 0..2000 {
            looper.process([0.8, 0.7, 0.6, 0.5, 0.4]);
        }
        assert!(looper.stop_recording(4));

        assert!((looper.parts[0].buffer[1000] - 0.1).abs() < 0.0001);
        assert!((looper.parts[1].buffer[1000] - 0.5).abs() < 0.0001);
        assert!((looper.parts[4].buffer[1000] - 0.4).abs() < 0.0001);
    }

    #[test]
    fn record_finish_commits_layer_starts_playback_and_sets_duration() {
        let mut looper = Looper::new();

        looper.start_recording(0);
        for _ in 0..2000 {
            looper.process([0.25, 0.0, 0.0, 0.0, 0.0]);
        }

        assert!(looper.stop_recording(0));
        assert_eq!(looper.master_duration, 2000);
        assert_eq!(looper.parts[0].layer_count(), 1);
        assert_eq!(looper.part_state(0), "playing");
        assert!(looper.parts[0].is_playing);
        assert!(looper.parts[0]
            .get_waveform(100)
            .iter()
            .any(|peak| *peak > 0.2));
    }

    #[test]
    fn silent_recording_is_rejected_as_empty_material() {
        let mut looper = Looper::new();

        looper.start_recording(0);
        for _ in 0..16 {
            looper.process([0.0; LOOPER_PART_COUNT]);
        }

        assert!(!looper.stop_recording(0));
        assert_eq!(looper.master_duration, 0);
        assert_eq!(looper.parts[0].layer_count(), 0);
        assert_eq!(looper.part_state(0), "empty");
        assert!(looper.parts[0].get_waveform(100).is_empty());
    }

    #[test]
    fn play_part_switches_to_only_the_active_part() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);
        looper.load_part_layers(1, vec![vec![0.2; 8]]);

        assert!(looper.play_part(0));
        assert!(looper.parts[0].is_playing);
        assert!(!looper.parts[1].is_playing);

        assert!(looper.play_part(1));
        assert!(!looper.parts[0].is_playing);
        assert!(looper.parts[1].is_playing);
        assert_eq!(looper.parts[1].play_ptr, 0);
    }

    #[test]
    fn play_all_recorded_starts_every_recorded_part() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);
        looper.load_part_layers(1, vec![vec![0.2; 8]]);

        assert!(looper.play_all_recorded());
        assert!(looper.parts[0].is_playing);
        assert!(looper.parts[1].is_playing);
        assert!(!looper.parts[2].is_playing);
        assert!(!looper.parts[3].is_playing);
        assert!(!looper.parts[4].is_playing);

        let per_part = looper.process([0.0; LOOPER_PART_COUNT]);
        assert!(per_part[0] > 0.0);
        assert!(per_part[1] > 0.0);
        assert_eq!(per_part[2], 0.0);
        assert_eq!(per_part[3], 0.0);
        assert_eq!(per_part[4], 0.0);
    }

    #[test]
    fn play_all_recorded_unmutes_previously_paused_parts() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);

        assert!(looper.play_all_recorded());
        assert_eq!(looper.toggle_part_active(0), Some(false));
        assert!(looper.parts[0].is_muted);
        looper.stop();

        assert!(looper.play_all_recorded());
        assert!(!looper.parts[0].is_muted);
        assert_eq!(looper.part_state(0), "playing");
        assert!(looper.process([0.0; LOOPER_PART_COUNT])[0] > 0.0);
    }

    #[test]
    fn playback_wraps_to_the_start_of_the_loop() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1, 0.2, 0.3]]);

        assert!(looper.play_all_recorded());

        let outputs = [
            looper.process([0.0; LOOPER_PART_COUNT])[0],
            looper.process([0.0; LOOPER_PART_COUNT])[0],
            looper.process([0.0; LOOPER_PART_COUNT])[0],
            looper.process([0.0; LOOPER_PART_COUNT])[0],
        ];
        assert_eq!(outputs, [0.1, 0.2, 0.3, 0.1]);
    }

    #[test]
    fn muted_part_keeps_clocking_and_returns_in_time() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);
        looper.load_part_layers(1, vec![vec![0.2; 8]]);

        assert!(looper.play_all_recorded());
        looper.process([0.0; LOOPER_PART_COUNT]);
        looper.process([0.0; LOOPER_PART_COUNT]);

        assert_eq!(looper.toggle_part_active(1), Some(false));
        let muted_ptr = looper.parts[1].play_ptr;
        let muted_output = looper.process([0.0; LOOPER_PART_COUNT]);
        assert!(muted_output[0] > 0.0);
        assert_eq!(muted_output[1], 0.0);
        assert_eq!(looper.parts[1].play_ptr, (muted_ptr + 1) % 8);

        looper.process([0.0; LOOPER_PART_COUNT]);
        assert_eq!(looper.toggle_part_active(1), Some(true));
        assert_eq!(looper.part_state(1), "playing");
        let resumed_output = looper.process([0.0; LOOPER_PART_COUNT]);
        assert!(resumed_output[1] > 0.0);
    }

    #[test]
    fn finalize_layer_does_not_auto_boost_quiet_recordings() {
        let mut layer = vec![0.02; 2000];

        LoopPart::finalize_layer(&mut layer);

        assert!(layer[1000] <= 0.021);
        assert!(layer[0].abs() < 0.001);
        assert!(layer[1999].abs() < 0.001);
    }

    #[test]
    fn overdub_writes_from_current_playhead_position() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);
        assert!(looper.play_part(0));
        looper.process([0.0; LOOPER_PART_COUNT]);
        looper.process([0.0; LOOPER_PART_COUNT]);

        looper.start_overdub(0);
        looper.process([0.001; LOOPER_PART_COUNT]);
        assert!(looper.stop_recording(0));

        assert_eq!(looper.parts[0].layer_count(), 2);
        assert_eq!(looper.parts[0].layers[1][0], 0.0);
        assert_eq!(looper.parts[0].layers[1][1], 0.0);
        assert!(looper.parts[0].layers[1][2] > 0.0);
    }

    #[test]
    fn silent_overdub_keeps_existing_loop_and_layer_count() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);
        assert!(looper.play_all_recorded());

        looper.start_overdub(0);
        for _ in 0..8 {
            looper.process([0.0; LOOPER_PART_COUNT]);
        }

        assert!(!looper.stop_recording(0));
        assert_eq!(looper.parts[0].layer_count(), 1);
        assert_eq!(looper.part_state(0), "playing");
        assert!(looper.process([0.0; LOOPER_PART_COUNT])[0] > 0.0);
    }

    #[test]
    fn clear_part_preserves_remaining_part_duration() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);
        looper.load_part_layers(1, vec![vec![0.2; 6]]);

        looper.clear_part(0);

        assert_eq!(looper.master_duration, 6);
        assert!(!looper.parts[0].has_material());
        assert!(looper.parts[1].has_material());
        assert_eq!(looper.parts[1].buffer, vec![0.2; 6]);
    }

    #[test]
    fn clear_last_part_resets_master_duration() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8]]);

        looper.clear_part(0);

        assert_eq!(looper.master_duration, 0);
        assert!(!looper.parts[0].has_material());
    }

    #[test]
    fn undo_and_clear_cancel_active_sequence() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 8], vec![0.2; 8]]);
        looper.load_part_layers(1, vec![vec![0.3; 8]]);

        assert!(looper.start_sequence(vec![0, 1]));
        assert!(looper.sequence_active);
        assert!(looper.undo_part(0));
        assert!(!looper.sequence_active);

        assert!(looper.start_sequence(vec![0, 1]));
        assert!(looper.sequence_active);
        looper.clear_part(1);
        assert!(!looper.sequence_active);
    }

    #[test]
    fn sequence_filters_empty_parts_and_wraps() {
        let mut looper = Looper::new();
        looper.load_part_layers(0, vec![vec![0.1; 4]]);
        looper.load_part_layers(2, vec![vec![0.3; 4]]);
        looper.load_part_layers(4, vec![vec![0.5; 4]]);

        assert!(looper.start_sequence(vec![0, 1, 2, 3, 4]));
        assert_eq!(looper.sequence, vec![0, 2, 4]);
        assert_eq!(looper.current_sequence_part(), Some(0));
        assert!(looper.parts[0].is_playing);

        for _ in 0..4 {
            looper.process([0.0; LOOPER_PART_COUNT]);
        }
        assert_eq!(looper.current_sequence_part(), Some(2));
        assert!(looper.parts[2].is_playing);

        for _ in 0..4 {
            looper.process([0.0; LOOPER_PART_COUNT]);
        }
        assert_eq!(looper.current_sequence_part(), Some(4));
        assert!(looper.parts[4].is_playing);

        for _ in 0..4 {
            looper.process([0.0; LOOPER_PART_COUNT]);
        }
        assert_eq!(looper.current_sequence_part(), Some(0));
        assert!(looper.parts[0].is_playing);
    }
}
