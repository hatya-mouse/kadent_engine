use crate::{
    data_types::MidiEvent,
    mixer::{Mixer, TrackID},
    thread::AudioCommand,
    track::note_track::NoteTrack,
};
use cpal::traits::DeviceTrait;
use ringbuf::{
    HeapCons,
    traits::{Consumer, Producer},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Clone)]
pub(super) struct OutputCallbackState {
    pub(super) playhead: Arc<AtomicUsize>,
    pub(super) is_playing: Arc<AtomicBool>,
}

pub(super) struct OutputCallbackContext {
    pub(super) command_cons: HeapCons<AudioCommand>,
    pub(super) midi_cons: ringbuf::HeapCons<MidiEvent>,
    pub(super) vu_prod: ringbuf::HeapProd<f32>,
    pub(super) latest_mixer: Arc<Mutex<Option<Mixer>>>,
}

pub(super) fn output_callback(
    ctx: Arc<Mutex<OutputCallbackContext>>,
    device: cpal::Device,
    config: cpal::StreamConfig,
    state: OutputCallbackState,
    initial_mixer: Mixer,
) -> cpal::Stream {
    let mut armed_track: Option<TrackID> = None;

    device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                let Ok(mut ctx) = ctx.try_lock() else {
                    return;
                };
                let Some(mixer) = ctx
                    .latest_mixer
                    .try_lock()
                    .ok()
                    .and_then(|mut mixer| mixer.as_mut())
                else {
                    return;
                };

                let mut current_playhead = state.playhead.load(Ordering::Relaxed);

                // Process all pending commands from the audio command ringbuf
                while let Some(command) = ctx.command_cons.try_pop() {
                    match command {
                        AudioCommand::Seek(target) => {
                            let target_sample = mixer.project.tempo_map.ticks_to_samples(target);
                            current_playhead = target_sample;
                            state.playhead.store(target_sample, Ordering::Relaxed);
                            mixer.seek(target_sample);
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
                let midi_events: Vec<MidiEvent> = ctx.midi_cons.pop_iter().collect();
                if !midi_events.is_empty()
                    && let Some(track_id) = armed_track
                    && let Some(track) = mixer.project.tracks.get_mut(&track_id)
                    && let Some(note_track) = track.as_any_mut().downcast_mut::<NoteTrack>()
                {
                    note_track.pass_midi(&midi_events);
                }

                let is_playing = state.is_playing.load(Ordering::Relaxed);

                // Process the audio and fill the output buffer
                mixer.process(is_playing, current_playhead, data);

                // Send the generated waveform data to the main thread for visualization
                let channels = mixer.project.audio_ctx.channels;
                for ch in 0..channels {
                    let rms = (data
                        .iter()
                        .step_by(channels)
                        .skip(ch)
                        .map(|x| x * x)
                        .sum::<f32>()
                        / (data.len() / channels) as f32)
                        .sqrt();
                    ctx.vu_prod.try_push(rms).ok();
                }

                if is_playing {
                    state
                        .playhead
                        .fetch_add(mixer.project.audio_ctx.buffer_size, Ordering::Relaxed);
                }
            },
            |err| {
                eprintln!("An error occured on stream: {}", err);
            },
            None,
        )
        .expect("Failed to create a new stream")
}
