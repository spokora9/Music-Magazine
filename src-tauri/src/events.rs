// SHED POWER: Tauri Events (Rust -> Svelte)

#[derive(Clone, serde::Serialize)]
pub struct WaveformPayload {
    pub part_id: usize,
    pub data: Vec<f32>, // Downsampled visual data (0.0 - 1.0)
}
