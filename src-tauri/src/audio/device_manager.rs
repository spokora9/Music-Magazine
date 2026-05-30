// SHED POWER: Device Hot-Plug Manager
use crate::audio::{AudioCommand, AudioThreadEvent, MidiHandler, MidiMap};
use cpal::traits::{DeviceTrait, HostTrait};
#[cfg(not(target_os = "android"))]
use midir::MidiInput;
use ringbuf::HeapProducer;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub id: String,
    pub device_type: DeviceType,
}

#[derive(Debug, Clone)]
pub enum DeviceType {
    #[cfg_attr(target_os = "android", allow(dead_code))]
    MidiInput,
    AudioInput,
    AudioOutput,
}

#[derive(Clone)]
pub struct DeviceManager {
    pub known_devices: Arc<Mutex<Vec<DeviceInfo>>>,
    pub midi_handler: Arc<Mutex<Option<MidiHandler>>>,
    pub command_producer: Arc<Mutex<HeapProducer<AudioCommand>>>,
    pub event_producer: Arc<Mutex<HeapProducer<AudioThreadEvent>>>,
    pub midi_map: Arc<Mutex<MidiMap>>,
    pub midi_active: Arc<AtomicBool>,
}

impl DeviceManager {
    pub fn new(
        command_producer: Arc<Mutex<HeapProducer<AudioCommand>>>,
        event_producer: Arc<Mutex<HeapProducer<AudioThreadEvent>>>,
        midi_map: Arc<Mutex<MidiMap>>,
        midi_active: Arc<AtomicBool>,
    ) -> Self {
        Self {
            known_devices: Arc::new(Mutex::new(Vec::new())),
            midi_handler: Arc::new(Mutex::new(None)),
            command_producer,
            event_producer,
            midi_map,
            midi_active,
        }
    }

    pub fn scan_devices(&self) -> anyhow::Result<Vec<DeviceInfo>> {
        let mut devices = Vec::new();

        // Scan MIDI Devices
        #[cfg(not(target_os = "android"))]
        if let Ok(midi_in) = MidiInput::new("shed-device-scan") {
            let ports = midi_in.ports();
            for (i, port) in ports.iter().enumerate() {
                if let Ok(name) = midi_in.port_name(port) {
                    devices.push(DeviceInfo {
                        name: name.clone(),
                        id: format!("midi_in_{}", i),
                        device_type: DeviceType::MidiInput,
                    });
                }
            }
        }

        // Scan Audio Devices
        let host = cpal::default_host();

        // Audio Input Devices
        if let Ok(input_devices) = host.input_devices() {
            for (i, device) in input_devices.enumerate() {
                if let Ok(name) = device.name() {
                    devices.push(DeviceInfo {
                        name: name.clone(),
                        id: format!("audio_in_{}", i),
                        device_type: DeviceType::AudioInput,
                    });
                }
            }
        }

        // Audio Output Devices
        if let Ok(output_devices) = host.output_devices() {
            for (i, device) in output_devices.enumerate() {
                if let Ok(name) = device.name() {
                    devices.push(DeviceInfo {
                        name: name.clone(),
                        id: format!("audio_out_{}", i),
                        device_type: DeviceType::AudioOutput,
                    });
                }
            }
        }

        Ok(devices)
    }

    pub fn check_for_changes(&self) -> anyhow::Result<(Vec<DeviceInfo>, Vec<DeviceInfo>)> {
        let current_devices = self.scan_devices()?;
        let mut known = self
            .known_devices
            .lock()
            .map_err(|_| anyhow::anyhow!("Lock failed"))?;

        // Find new devices
        let new_devices: Vec<DeviceInfo> = current_devices
            .iter()
            .filter(|device| {
                !known
                    .iter()
                    .any(|known_device| known_device.id == device.id)
            })
            .cloned()
            .collect();

        // Find removed devices
        let removed_devices: Vec<DeviceInfo> = known
            .iter()
            .filter(|known_device| {
                !current_devices
                    .iter()
                    .any(|device| device.id == known_device.id)
            })
            .cloned()
            .collect();

        // Update known devices
        *known = current_devices;

        Ok((new_devices, removed_devices))
    }

    pub fn refresh_midi(&self) -> anyhow::Result<()> {
        println!("DeviceManager: Refreshing MIDI connections...");

        // Create new MIDI handler
        let debug_rb = ringbuf::HeapRb::new(1024);
        let (debug_prod, _) = debug_rb.split();
        let midi_debug_producer = Arc::new(Mutex::new(debug_prod));

        match MidiHandler::new(
            self.command_producer.clone(),
            self.midi_map.clone(),
            self.midi_active.clone(),
            midi_debug_producer,
        ) {
            Ok(new_handler) => {
                let mut handler_lock = self
                    .midi_handler
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Lock failed"))?;
                *handler_lock = Some(new_handler);

                println!("DeviceManager: MIDI refresh successful");
                Ok(())
            }
            Err(e) => {
                println!("DeviceManager: MIDI refresh failed: {}", e);

                // Set to dummy handler
                let mut handler_lock = self
                    .midi_handler
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Lock failed"))?;
                *handler_lock = Some(MidiHandler::dummy());

                Err(e)
            }
        }
    }

    pub fn refresh_audio(&self) -> anyhow::Result<()> {
        println!("DeviceManager: Audio refresh requested (requires engine restart)");

        // For audio refresh, we would need to restart the entire audio engine
        // This is more complex and risky during runtime, so for now we'll just scan
        let devices = self.scan_devices()?;
        let audio_devices: Vec<&DeviceInfo> = devices
            .iter()
            .filter(|d| {
                matches!(
                    d.device_type,
                    DeviceType::AudioInput | DeviceType::AudioOutput
                )
            })
            .collect();

        println!("DeviceManager: Found {} audio devices", audio_devices.len());
        for device in audio_devices {
            println!("  - {}: {}", device.id, device.name);
        }

        // Emit device list update
        if let Ok(mut prod) = self.event_producer.lock() {
            let _ = prod.push(AudioThreadEvent::DeviceListUpdate);
        }

        Ok(())
    }
}
