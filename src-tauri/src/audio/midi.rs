// SHED POWER: MIDI Input Listener
#[cfg(not(target_os = "android"))]
use crate::audio::LooperSource;
use crate::audio::{AudioCommand, MidiMap, MidiTarget};
#[cfg(not(target_os = "android"))]
use crate::persistence;
#[cfg(not(target_os = "android"))]
use midir::{Ignore, MidiInput};
use ringbuf::HeapProducer;
use std::sync::atomic::AtomicBool;
#[cfg(not(target_os = "android"))]
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

pub struct MidiHandler {
    #[cfg(not(target_os = "android"))]
    _conn: Option<midir::MidiInputConnection<()>>,
    #[cfg(target_os = "android")]
    _conn: Option<()>,
    pub learning_target: Arc<Mutex<Option<MidiTarget>>>,
    pub _is_active: Arc<AtomicBool>,
    pub _debug_producer: Arc<Mutex<HeapProducer<crate::audio::AudioThreadEvent>>>,
}

unsafe impl Send for MidiHandler {}
unsafe impl Sync for MidiHandler {}

#[cfg(not(target_os = "android"))]
fn persist_looper_source_command(cmd: &AudioCommand) {
    let source_update = match cmd {
        AudioCommand::SetLooperPartSource { part_id, source } => Some((*part_id, *source)),
        AudioCommand::SetLooperPartInput { part_id, channel } => {
            let source = if *channel == 0 {
                LooperSource::InputMix
            } else {
                LooperSource::InputChannel((*channel).min(8))
            };
            Some((*part_id, source))
        }
        _ => None,
    };

    if let Some((part_id, source)) = source_update {
        let _ = persistence::update_app_persistence(|data| {
            data.settings.looper_sources =
                persistence::normalize_looper_sources(&data.settings.looper_sources);
            if let Some(slot) = data.settings.looper_sources.get_mut(part_id) {
                *slot = source;
            }
        });
    }
}

impl MidiHandler {
    pub fn dummy() -> Self {
        // Create a dummy ringbuf just to satisfy the struct
        let rb = ringbuf::HeapRb::new(1);
        let (p, _) = rb.split();
        Self {
            _conn: None,
            learning_target: Arc::new(Mutex::new(None)),
            _is_active: Arc::new(AtomicBool::new(true)),
            _debug_producer: Arc::new(Mutex::new(p)),
        }
    }

    pub fn new(
        command_producer: Arc<Mutex<HeapProducer<AudioCommand>>>,
        midi_map: Arc<Mutex<MidiMap>>,
        is_active: Arc<AtomicBool>,
        debug_producer: Arc<Mutex<HeapProducer<crate::audio::AudioThreadEvent>>>,
    ) -> anyhow::Result<Self> {
        let learning_target = Arc::new(Mutex::new(None));

        #[cfg(target_os = "android")]
        {
            let _ = command_producer;
            let _ = midi_map;
            println!("MIDI: native MIDI backend disabled on Android; using dummy handler.");
            return Ok(Self {
                _conn: None,
                learning_target,
                _is_active: is_active,
                _debug_producer: debug_producer,
            });
        }

        #[cfg(not(target_os = "android"))]
        {
            // 1. Discovery Phase
            let temp_midi = MidiInput::new("shed-discovery")?;
            let ports = temp_midi.ports();
            let mut candidates = Vec::new();

            println!("MIDI: Discovery found {} ports", ports.len());
            for (i, p) in ports.iter().enumerate() {
                let name = temp_midi
                    .port_name(p)
                    .unwrap_or_else(|_| "Unknown".to_string());
                let lower = name.to_lowercase();
                let is_priority = lower.contains("mpk")
                    || lower.contains("akai")
                    || lower.contains("key")
                    || lower.contains("usb")
                    || lower.contains("midi");
                println!("  [{}] {} (Priority: {})", i, name, is_priority);
                candidates.push((i, name, is_priority));
            }

            // Sort: Priority (true) comes first
            candidates.sort_by(|a, b| b.2.cmp(&a.2));

            // 2. Connection Phase
            for (index, name, _) in candidates {
                println!("MIDI: Attempting connection to '{}'...", name);

                // Re-create MidiInput for each attempt (as connect() consumes it)
                match MidiInput::new("shed-power-input") {
                    Ok(mut midi_in) => {
                        midi_in.ignore(Ignore::None);
                        let fresh_ports = midi_in.ports();

                        if let Some(port) = fresh_ports.get(index) {
                            // Prepare callback closures
                            let active_clone = is_active.clone();
                            let debug_clone = debug_producer.clone();
                            let learn_clone = learning_target.clone();
                            let map_clone = midi_map.clone();
                            let cmd_producer_clone = command_producer.clone();

                            // Attempt Connect with Inline Closure
                            match midi_in.connect(port, "shed-power-read", move |_, message, _| {
                            if !active_clone.load(Ordering::Relaxed) { return; }
                            if message.len() < 3 { return; }

                            let status = message[0] & 0xF0;
                            let data1 = message[1];
                            let data2 = message[2];

                            if let Ok(mut prod) = debug_clone.lock() {
                                let _ = prod.push(crate::audio::AudioThreadEvent::MidiDebug { status, data1, data2 });
                            }

                            // --- MIDI LEARN & MAPPING LOGIC START ---
                            if let Ok(mut target_lock) = learn_clone.lock() {
                                if let Some(target) = *target_lock {
                                    let mut learned_snapshot = None;
                                    if let Ok(mut map) = map_clone.lock() {
                                        let ctx = map.active_context.clone();
                                        match status {
                                            0xB0 => {
                                                println!("MIDI LEARN: Mapping CC {} to {:?} (Context: {})", data1, target, ctx);
                                                map.cc_maps.entry(ctx).or_default().insert(data1, target);
                                                learned_snapshot = Some(map.clone());
                                            },
                                            0x90 => {
                                                println!("MIDI LEARN: Mapping Note {} to {:?} (Context: {})", data1, target, ctx);
                                                map.note_maps.entry(ctx).or_default().insert(data1, target);
                                                learned_snapshot = Some(map.clone());
                                            },
                                            _ => {}
                                        }
                                    }
                                    if let Some(snapshot) = learned_snapshot {
                                        let _ = persistence::update_app_persistence(|data| {
                                            data.midi_map = snapshot;
                                        });
                                    }
                                    *target_lock = None;
                                    return;
                                }
                            }

                            let target = if let Ok(map) = map_clone.lock() {
                                map.get_target(status, data1)
                            } else {
                                None
                            };

                            if let Some(t) = target {
                                println!("MIDI TARGET RESOLVED: {:?} (Data2: {})", t, data2);
                            }

                            let is_trigger = (status == 0x90 && data2 > 0) || (status == 0xB0 && data2 >= 64);

                            let cmd = if let Some(t) = target {
                                match t {
                                    MidiTarget::Param(id) => { let val = data2 as f32 / 127.0; Some(AudioCommand::SetParam { id, value: val }) },
                                    MidiTarget::Note(note_id) => {
                                        if status == 0x80 || (status == 0x90 && data2 == 0) { Some(AudioCommand::NoteOff { note: note_id }) }
                                        else if status == 0x90 && data2 > 0 { Some(AudioCommand::NoteOn { note: note_id, velocity: data2 }) }
                                        else { None }
                                    },
                                    MidiTarget::MpcParam(id) => { let val = data2 as f32 / 127.0; Some(AudioCommand::SetMpcParam { id, value: val }) },
                                    MidiTarget::Transport(id) => if is_trigger { match id {
                                        0 => Some(AudioCommand::TogglePlayback),
                                        1 => Some(AudioCommand::Stop),
                                        2 => Some(AudioCommand::Record { part_id: 0 }),
                                        _ => None
                                    } } else { None },
                                    MidiTarget::LooperRecord(id) => if is_trigger { Some(AudioCommand::Record { part_id: id }) } else { None },
                                    MidiTarget::LooperOverdub(id) => if is_trigger { Some(AudioCommand::Overdub { part_id: id }) } else { None },
                                    MidiTarget::LooperToggle(id) => if is_trigger { Some(AudioCommand::ToggleLooper { part_id: id }) } else { None },
                                    MidiTarget::LooperSelect(id) => if is_trigger { Some(AudioCommand::SelectPart { part_id: id }) } else { None },
                                    MidiTarget::LooperClear(id) => if is_trigger { Some(AudioCommand::ClearPart { part_id: id }) } else { None },
                                    MidiTarget::LooperUndo(id) => if is_trigger { Some(AudioCommand::Undo { part_id: id }) } else { None },
                                    MidiTarget::LooperActiveToggle(id) => if is_trigger { Some(AudioCommand::ToggleLooperPartActive { part_id: id }) } else { None },
                                    MidiTarget::LooperSourceInput { part_id, channel } => if is_trigger { Some(AudioCommand::SetLooperPartInput { part_id, channel }) } else { None },
                                    MidiTarget::LooperSource { part_id, source } => if is_trigger { Some(AudioCommand::SetLooperPartSource { part_id, source }) } else { None },
                                    MidiTarget::MicGain => { let gain = (data2 as f32 / 127.0) * 5.0; Some(AudioCommand::SetMicGain { gain }) },
                                    MidiTarget::JamPlay => if is_trigger { Some(AudioCommand::JamControl { action: 0 }) } else { None },
                                    MidiTarget::JamStop => if is_trigger { Some(AudioCommand::JamControl { action: 1 }) } else { None },
                                    MidiTarget::JamNext => if is_trigger { Some(AudioCommand::JamControl { action: 2 }) } else { None },
                                    MidiTarget::JamPrev => if is_trigger { Some(AudioCommand::JamControl { action: 3 }) } else { None },
                                }
                            } else {
                                // Unmapped fallback: Context Dependent
                                // Only Synth and MPC should play notes by default.
                                // Looper and Jam should be silent unless mapped.
                                let ctx = map_clone.lock().map(|m| m.active_context.clone()).unwrap_or("synth".to_string());

                                if ctx == "synth" || ctx == "mpc" {
                                    match status {
                                        0x90 => {
                                            if data2 > 0 {
                                                Some(AudioCommand::NoteOn { note: data1, velocity: data2 })
                                            } else {
                                                Some(AudioCommand::NoteOff { note: data1 })
                                            }
                                        },
                                        0x80 => Some(AudioCommand::NoteOff { note: data1 }),
                                        _ => None
                                    }
                                } else {
                                    None
                                }
                            };

                            if let Some(c) = cmd {
                                persist_looper_source_command(&c);
                                if let Ok(mut producer) = cmd_producer_clone.lock() {
                                    let _ = producer.push(c);
                                }
                            }
                            // --- MIDI MAPPING END ---
                        }, ()) {
                            Ok(conn) => {
                                println!("MIDI: SUCCESS! Connected to {}", name);
                                return Ok(Self {
                                    _conn: Some(conn),
                                    learning_target,
                                    _is_active: is_active,
                                    _debug_producer: debug_producer,
                                });
                            },
                            Err(e) => {
                                println!("MIDI: Failed to connect to {}: {}", name, e);
                                // Continue loop
                            }
                        }
                        }
                    }
                    Err(e) => println!("MIDI: Error creating MidiInput: {}", e),
                }
            }

            println!("MIDI: All connection attempts failed.");
            Ok(Self {
                _conn: None,
                learning_target,
                _is_active: is_active,
                _debug_producer: debug_producer,
            })
        }
    }
}
