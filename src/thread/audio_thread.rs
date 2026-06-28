use crate::{
    data_types::{AudioContext, MidiEvent},
    mixer::{Mixer, Project},
    thread::{
        AudioCommand, AudioError, AudioResult, export,
        output_callback::{OutputCallbackContext, OutputCallbackState, output_callback},
    },
};
use cpal::traits::{HostTrait, StreamTrait};
use ringbuf::traits::{Consumer, Producer, Split};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};

pub(super) fn audio_thread(
    command_rx: mpsc::Receiver<AudioCommand>,
    result_tx: mpsc::Sender<Result<AudioResult, AudioError>>,
    mut midi_cons: ringbuf::HeapCons<MidiEvent>,
    vu_prod: ringbuf::HeapProd<f32>,
    playhead: Arc<AtomicUsize>,
    audio_ctx: AudioContext,
    initial_project: Project,
) {
    let (mut command_prod, command_cons) = ringbuf::HeapRb::<AudioCommand>::new(64).split();
    let (mut midi_sub_prod, midi_sub_cons) = ringbuf::HeapRb::<MidiEvent>::new(64).split();

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
    let callback_ctx = Arc::new(Mutex::new(OutputCallbackContext {
        mixer,
        command_cons,
        midi_cons: midi_sub_cons,
        vu_prod,
        pending_project: pending_arc,
    }));
    let callback_state = OutputCallbackState {
        playhead,
        is_playing: is_playing_clone,
    };
    let mut stream = Some(output_callback(
        callback_ctx.clone(),
        device,
        config,
        callback_state.clone(),
    ));

    if let Some(stream) = stream.as_ref()
        && let Err(err) = stream.play()
    {
        result_tx
            .send(Err(AudioError::PlayStreamError(err)))
            .unwrap();
    }

    // Create a message loop
    loop {
        while let Ok(command) = command_rx.try_recv() {
            match command {
                AudioCommand::Play => {
                    is_playing.store(true, Ordering::Release);
                }
                AudioCommand::Pause => {
                    is_playing.store(false, Ordering::Release);
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
                AudioCommand::SetOutputDevice(device) => {
                    stream.take();

                    // Create a new MIDI ring buffer and split it into producer and consumer
                    let (new_sub_prod, new_sub_cons) =
                        ringbuf::HeapRb::<MidiEvent>::new(64).split();
                    midi_sub_prod = new_sub_prod;

                    callback_ctx.lock().unwrap().midi_cons = new_sub_cons;

                    stream = Some(output_callback(
                        callback_ctx.clone(),
                        device,
                        config,
                        callback_state.clone(),
                    ));

                    if let Some(stream) = stream.as_ref()
                        && let Err(err) = stream.play()
                    {
                        result_tx
                            .send(Err(AudioError::PlayStreamError(err)))
                            .unwrap();
                    }
                }
                AudioCommand::Seek(_) | AudioCommand::ArmTrack(_) | AudioCommand::DisarmTrack => {
                    if let Err(command) = command_prod.try_push(command) {
                        result_tx
                            .send(Err(AudioError::CommandFailed(command)))
                            .unwrap();
                    }
                }
            }
        }

        // Send the MIDI events from the midi_cons to the midi_sub_prod
        while let Some(midi_event) = midi_cons.try_pop() {
            midi_sub_prod.try_push(midi_event).ok();
        }
    }
}
