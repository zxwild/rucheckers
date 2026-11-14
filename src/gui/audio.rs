//! Sound playback. The signals are synthesized in the library (`rucheckers::sound`);
//! here they are only sent to the sound device.
//!
//! The device is opened in a separate thread at the first sound. If there is none, or
//! it is busy, the game goes on without sound.

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle};
use rucheckers::sound::{synthesize, Sound, SAMPLE_RATE};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::{Duration, Instant};

/// Pause between attempts to open the sound device after a failure.
const RETRY_AFTER: Duration = Duration::from_secs(10);

/// Sound player.
pub struct Audio {
    sender: Option<Sender<Sound>>,
}

impl Audio {
    /// Creates a player. The sound device is not opened yet.
    pub fn new() -> Self {
        let (sender, receiver) = channel::<Sound>();
        let worker = move || {
            let bank: HashMap<Sound, Vec<f32>> = Sound::ALL.iter().map(|&s| (s, synthesize(s))).collect();
            // The output stream must live while a sound plays, and it cannot leave its thread.
            let mut output: Option<(OutputStream, OutputStreamHandle)> = None;
            let mut last_failure: Option<Instant> = None;
            for sound in receiver {
                if output.is_none() && last_failure.is_none_or(|at| at.elapsed() >= RETRY_AFTER) {
                    match OutputStream::try_default() {
                        Ok(pair) => output = Some(pair),
                        Err(_) => last_failure = Some(Instant::now()),
                    }
                }
                let Some((_, handle)) = &output else { continue };
                let Some(samples) = bank.get(&sound) else { continue };
                let source = SamplesBuffer::new(1, SAMPLE_RATE, samples.clone());
                if handle.play_raw(source).is_err() {
                    // The device is gone: it will be opened again at the next sound.
                    output = None;
                    last_failure = Some(Instant::now());
                }
            }
        };
        let spawned = thread::Builder::new()
            .name("rucheckers-audio".to_string())
            .spawn(worker);
        Self {
            sender: spawned.ok().map(|_| sender),
        }
    }

    /// Queues a sound for playback.
    pub fn play(&self, sound: Sound) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(sound);
        }
    }
}
