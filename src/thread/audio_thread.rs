use crate::{
    data_types::{AudioContext, MidiEvent},
    mixer::{Mixer, Project, TrackID},
    thread::{AudioCommand, AudioError, AudioResult, export},
    track::note_track::NoteTrack,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{
    SharedRb,
    storage::Heap,
    traits::{Consumer, Producer, Split},
    wrap::caching::Caching,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};

pub(super) struct OutputCallbackState {
    pub(super) playhead: Arc<AtomicUsize>,
    pub(super) is_playing: Arc<AtomicBool>,
}

struct OutputCallbackContext {
    mixer: Mixer,
    consumer: Caching<Arc<SharedRb<Heap<AudioCommand>>>, false, true>,
    midi_consumer: ringbuf::HeapCons<MidiEvent>,
    vu_producer: ringbuf::HeapProd<f32>,
    pending_project: Arc<Mutex<Option<Project>>>,
}

pub(super) fn audio_thread(
    command_rx: mpsc::Receiver<AudioCommand>,
    result_tx: mpsc::Sender<Result<AudioResult, AudioError>>,
    midi_consumer: ringbuf::HeapCons<MidiEvent>,
    vu_producer: ringbuf::HeapProd<f32>,
    playhead: Arc<AtomicUsize>,
    audio_ctx: AudioContext,
    initial_project: Project,
) {
    let (mut producer, consumer) = ringbuf::HeapRb::<AudioCommand>::new(64).split();

    // Create a mixer with the given initial project
    let pending_project = Arc::new(Mutex::new(None));
    let pending_arc = Arc::clone(&pending_project);
    let mixer = Mixer::new(initial_project);

    // Create a generation variable to track the latest prepared project
    let generation = Arc::new(AtomicUsize::new(0));

    // Get a cpal device
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Expect a default output device");

    // Manage is_playing using Arc
    let is_playing = Arc::new(AtomicBool::new(false));
    let is_playing_clone = is_playing.clone();

    // Create an output callback
    let config = cpal::StreamConfig {
        channels: audio_ctx.channels as u16,
        sample_rate: audio_ctx.sample_rate as u32,
        buffer_size: cpal::BufferSize::Fixed(audio_ctx.buffer_size as u32),
    };
    let callback_state = OutputCallbackState {
        playhead,
        is_playing: is_playing_clone,
    };
    let stream = output_callback(
        OutputCallbackContext {
            mixer,
            consumer,
            midi_consumer,
            vu_producer,
            pending_project: pending_arc,
        },
        device,
        config,
        callback_state,
    );

    if let Err(err) = stream.play() {
        result_tx
            .send(Err(AudioError::PlayStreamError(err)))
            .unwrap();
    }

    // Create a message loop
    for command in command_rx {
        match command {
            AudioCommand::Play => {
                is_playing.store(true, Ordering::Release);
            }
            AudioCommand::Pause => {
                is_playing.store(false, Ordering::Release);
            }
            AudioCommand::Seek(_) => {
                if let Err(command) = producer.try_push(command) {
                    result_tx
                        .send(Err(AudioError::CommandFailed(command)))
                        .unwrap();
                }
            }
            AudioCommand::UpdateProject(mut new_project) => {
                // Increment the current generation by one to mark it as the latest
                let current_gen = generation.fetch_add(1, Ordering::SeqCst) + 1;
                let gen_arc = Arc::clone(&generation);
                let pending_arc = Arc::clone(&pending_project);
                let result_tx = result_tx.clone();
                std::thread::spawn(move || {
                    // Prepare the project before applying the project
                    if let Err(err) = new_project.prepare() {
                        result_tx.send(Err(AudioError::GraphError(err))).unwrap();
                        return;
                    }

                    // Check if the project is the latest one
                    if gen_arc.load(Ordering::SeqCst) == current_gen {
                        // Send the new project to the audio playback thread
                        *pending_arc.lock().unwrap() = Some(*new_project);
                    }
                });
            }
            AudioCommand::ExportAudio(project) => {
                let result_tx = result_tx.clone();
                export::spawn_export_thread(result_tx, *project);
            }
            AudioCommand::ArmTrack(_) => {
                if let Err(command) = producer.try_push(command) {
                    result_tx
                        .send(Err(AudioError::CommandFailed(command)))
                        .unwrap();
                }
            }
            AudioCommand::DisarmTrack => {
                if let Err(command) = producer.try_push(command) {
                    result_tx
                        .send(Err(AudioError::CommandFailed(command)))
                        .unwrap();
                }
            }
        }
    }

    drop(stream);
}

fn output_callback(
    mut context: OutputCallbackContext,
    device: cpal::Device,
    config: cpal::StreamConfig,
    state: OutputCallbackState,
) -> cpal::Stream {
    let mut armed_track: Option<TrackID> = None;

    device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                let mut current_playhead = state.playhead.load(Ordering::Relaxed);

                // Get the project without blocking
                if let Ok(mut pending) = context.pending_project.try_lock()
                    && let Some(new_project) = pending.take()
                {
                    context.mixer.apply_project(new_project, current_playhead);
                }

                // Process all pending commands from the audio command ringbuf
                while let Some(command) = context.consumer.try_pop() {
                    match command {
                        AudioCommand::Seek(target) => {
                            let target_sample =
                                context.mixer.project.tempo_map.beats_to_samples(target);
                            current_playhead = target_sample;
                            state.playhead.store(target_sample, Ordering::Relaxed);
                            context.mixer.seek(target_sample);
                        }
                        AudioCommand::ArmTrack(track_id) => {
                            armed_track = Some(track_id);
                        }
                        AudioCommand::DisarmTrack => {
                            armed_track = None;
                        }
                        _ => {}
                    }
                }

                // Drain MIDI events and pass them to the armed NoteTrack
                let midi_events: Vec<MidiEvent> = context.midi_consumer.pop_iter().collect();
                if !midi_events.is_empty()
                    && let Some(track_id) = armed_track
                    && let Some(track) = context.mixer.project.tracks.get_mut(&track_id)
                    && let Some(note_track) = track.as_any_mut().downcast_mut::<NoteTrack>()
                {
                    note_track.pass_midi(&midi_events);
                }

                let is_playing = state.is_playing.load(Ordering::Relaxed);

                // Process the audio and fill the output buffer
                context.mixer.process(is_playing, current_playhead, data);

                // Send the generated waveform data to the main thread for visualization
                let channels = context.mixer.project.audio_ctx.channels;
                for ch in 0..channels {
                    let rms = (data
                        .iter()
                        .step_by(channels)
                        .skip(ch)
                        .map(|x| x * x)
                        .sum::<f32>()
                        / (data.len() / channels) as f32)
                        .sqrt();
                    context.vu_producer.try_push(rms).ok();
                }

                if is_playing {
                    state.playhead.fetch_add(
                        context.mixer.project.audio_ctx.buffer_size,
                        Ordering::Relaxed,
                    );
                }
            },
            |err| {
                eprintln!("An error occured on stream: {}", err);
            },
            None,
        )
        .expect("Failed to create a new stream")
}
