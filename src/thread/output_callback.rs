use crate::{
    data_types::MidiEvent,
    mixer::{Mixer, Project, TrackID},
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
use std::time::Instant;

#[derive(Clone)]
pub(super) struct OutputCallbackState {
    pub(super) playhead: Arc<AtomicUsize>,
    pub(super) is_playing: Arc<AtomicBool>,
}

pub(super) struct OutputCallbackContext {
    pub(super) mixer: Mixer,
    pub(super) command_cons: HeapCons<AudioCommand>,
    pub(super) midi_cons: ringbuf::HeapCons<MidiEvent>,
    pub(super) vu_prod: ringbuf::HeapProd<f32>,
    pub(super) pending_project: Arc<Mutex<Option<Project>>>,
}

pub(super) fn output_callback(
    ctx: Arc<Mutex<OutputCallbackContext>>,
    device: cpal::Device,
    config: cpal::StreamConfig,
    state: OutputCallbackState,
) -> cpal::Stream {
    let mut armed_track: Option<TrackID> = None;
    let mut callback_count: u64 = 0;

    device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                let callback_start = Instant::now();
                callback_count += 1;

                let Ok(mut ctx) = ctx.try_lock() else {
                    return;
                };

                let mut current_playhead = state.playhead.load(Ordering::Relaxed);

                // Get the project without blocking
                if let Some(new_project) = ctx
                    .pending_project
                    .try_lock()
                    .ok()
                    .and_then(|mut pending| pending.take())
                {
                    ctx.mixer.apply_project(new_project, current_playhead);
                }

                // Process all pending commands from the audio command ringbuf
                while let Some(command) = ctx.command_cons.try_pop() {
                    match command {
                        AudioCommand::Seek(target) => {
                            let target_sample =
                                ctx.mixer.project.tempo_map.ticks_to_samples(target);
                            current_playhead = target_sample;
                            state.playhead.store(target_sample, Ordering::Relaxed);
                            ctx.mixer.seek(target_sample);
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
                    && let Some(track) = ctx.mixer.project.tracks.get_mut(&track_id)
                    && let Some(note_track) = track.as_any_mut().downcast_mut::<NoteTrack>()
                {
                    note_track.pass_midi(&midi_events);
                }

                let is_playing = state.is_playing.load(Ordering::Relaxed);

                // Process the audio and fill the output buffer
                let process_start = Instant::now();
                ctx.mixer.process(is_playing, current_playhead, data);
                let process_elapsed = process_start.elapsed();

                // Send the generated waveform data to the main thread for visualization
                let channels = ctx.mixer.project.audio_ctx.channels;
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
                        .fetch_add(ctx.mixer.project.audio_ctx.buffer_size, Ordering::Relaxed);
                }

                // 100コールバックごとに処理時間を報告する
                if callback_count % 100 == 0 {
                    let audio_ctx = &ctx.mixer.project.audio_ctx;
                    let deadline_us =
                        audio_ctx.buffer_size as f64 / audio_ctx.sample_rate as f64 * 1_000_000.0;
                    let total_us = callback_start.elapsed().as_micros() as f64;
                    let process_us = process_elapsed.as_micros() as f64;
                    eprintln!(
                        "[kadent] callback #{callback_count:>6}  \
                         deadline={deadline_us:.0}µs  \
                         total={total_us:.0}µs ({:.1}%)  \
                         process={process_us:.0}µs ({:.1}%)",
                        total_us / deadline_us * 100.0,
                        process_us / deadline_us * 100.0,
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
