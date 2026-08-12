use crate::{
    data_types::{MidiEvent, PlaybackContext},
    mixer::Mixer,
    thread::{
        AudioCommand, AudioError, AudioResult, export,
        output_callback::{OutputCallbackContext, OutputCallbackState, output_callback},
    },
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Consumer, Producer, Split};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering},
    mpsc,
};

pub(super) fn audio_thread(
    command_rx: mpsc::Receiver<AudioCommand>,
    result_tx: mpsc::Sender<Result<AudioResult, AudioError>>,
    mut midi_cons: ringbuf::HeapCons<MidiEvent>,
    vu_prod: ringbuf::HeapProd<f32>,
    playhead: Arc<AtomicU64>,
    playhead_ticks: Arc<AtomicI64>,
    playback_ctx: PlaybackContext,
) {
    let (mut command_prod, command_cons) = ringbuf::HeapRb::<AudioCommand>::new(64).split();
    let (mut midi_sub_prod, midi_sub_cons) = ringbuf::HeapRb::<MidiEvent>::new(64).split();

    // The latest playback context, tied to the latest prepared mixer.
    let mut latest_playback_ctx = playback_ctx;
    // A variable to hold the latest prepared mixer, which will be used by the output callback
    let latest_mixer = Arc::new(Mutex::new(None));
    // Manage is_playing using Arc
    let is_playing = Arc::new(AtomicBool::new(false));
    // Create a generation variable to keep track of the latest prepared mixer
    let generation = Arc::new(AtomicUsize::new(0));

    // Get a cpal device
    let host = cpal::default_host();
    let mut current_device = host
        .default_output_device()
        .expect("Expect a default output device");

    // Create an output callback
    let mut project_config = cpal::StreamConfig {
        channels: latest_playback_ctx.channels as u16,
        sample_rate: latest_playback_ctx.sample_rate as u32,
        buffer_size: cpal::BufferSize::Fixed(latest_playback_ctx.buffer_size as u32),
    };
    let callback_ctx = Arc::new(Mutex::new(OutputCallbackContext {
        command_cons,
        midi_cons: midi_sub_cons,
        vu_prod,
        result_tx: result_tx.clone(),
    }));
    let callback_state = OutputCallbackState {
        playhead,
        playhead_ticks,
        is_playing: is_playing.clone(),
    };
    let output_config = current_device
        .default_output_config()
        .map(|config| config.config())
        .unwrap_or(project_config);
    let mut stream = Some(output_callback(
        callback_ctx.clone(),
        current_device.clone(),
        output_config,
        callback_state.clone(),
        latest_mixer.clone(),
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
                AudioCommand::UpdateProject(new_project) => {
                    project_config = cpal::StreamConfig {
                        channels: latest_playback_ctx.channels as u16,
                        sample_rate: latest_playback_ctx.sample_rate as u32,
                        buffer_size: cpal::BufferSize::Fixed(
                            latest_playback_ctx.buffer_size as u32,
                        ),
                    };

                    // Increment the current generation by one to mark it as the latest
                    let current_gen = generation.fetch_add(1, Ordering::SeqCst) + 1;

                    // Copy the required variables to move into the thread
                    let gen_arc = Arc::clone(&generation);
                    let latest_arc = Arc::clone(&latest_mixer);
                    let result_tx = result_tx.clone();
                    let playback_ctx = latest_playback_ctx.clone();
                    std::thread::spawn(move || {
                        // Prepare the project before applying the project
                        match new_project.prepare(playback_ctx) {
                            Ok(mixer) => {
                                // Check if the mixer is the latest one
                                if gen_arc.load(Ordering::SeqCst) == current_gen {
                                    // Send the prepared mixer to the audio playback thread
                                    *latest_arc.lock().unwrap() = Some(mixer);
                                }
                            }
                            Err(err) => {
                                result_tx.send(Err(AudioError::GraphError(err))).unwrap();
                            }
                        }
                    });
                }
                AudioCommand::ExportAudio(project, playback_ctx) => {
                    let result_tx = result_tx.clone();
                    export::spawn_export_thread(result_tx, *project, playback_ctx);
                }
                AudioCommand::SetOutputDevice(device) => {
                    current_device = device;
                    recreate_output_callback(
                        &mut stream,
                        project_config,
                        &current_device,
                        &callback_ctx,
                        callback_state.clone(),
                        &mut midi_sub_prod,
                        &latest_mixer,
                    );

                    if let Some(stream) = stream.as_ref()
                        && let Err(err) = stream.play()
                    {
                        result_tx
                            .send(Err(AudioError::PlayStreamError(err)))
                            .unwrap();
                    }
                }
                AudioCommand::SetPlaybackCtx(new_playback_ctx) => {
                    latest_playback_ctx = new_playback_ctx;
                    recreate_output_callback(
                        &mut stream,
                        project_config,
                        &current_device,
                        &callback_ctx,
                        callback_state.clone(),
                        &mut midi_sub_prod,
                        &latest_mixer,
                    );

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

fn recreate_output_callback(
    stream: &mut Option<cpal::Stream>,
    project_config: cpal::StreamConfig,
    current_device: &cpal::Device,
    callback_ctx: &Arc<Mutex<OutputCallbackContext>>,
    callback_state: OutputCallbackState,
    midi_sub_prod: &mut ringbuf::HeapProd<MidiEvent>,
    latest_mixer: &Arc<Mutex<Option<Mixer>>>,
) {
    stream.take();

    // Create a new MIDI ring buffer and split it into producer and consumer
    let (new_sub_prod, new_sub_cons) = ringbuf::HeapRb::<MidiEvent>::new(64).split();
    *midi_sub_prod = new_sub_prod;

    callback_ctx.lock().unwrap().midi_cons = new_sub_cons;

    // Create a config
    let output_config = current_device
        .default_output_config()
        .map(|config| config.config())
        .unwrap_or(project_config);
    // Then get the latest mixer to pass to the new output callback
    *stream = Some(output_callback(
        callback_ctx.clone(),
        current_device.clone(),
        output_config,
        callback_state.clone(),
        latest_mixer.clone(),
    ));
}
