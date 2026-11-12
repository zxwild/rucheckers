//! Воспроизведение звуков. Сигналы синтезируются в библиотеке (`rucheckers::sound`),
//! здесь они только отправляются на звуковое устройство.
//!
//! Устройство открывается в отдельном потоке при первом звуке. Если его нет или оно
//! занято, игра продолжается без звука.

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle};
use rucheckers::sound::{synthesize, Sound, SAMPLE_RATE};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::{Duration, Instant};

/// Пауза между попытками открыть звуковое устройство после неудачи.
const RETRY_AFTER: Duration = Duration::from_secs(10);

/// Проигрыватель звуков.
pub struct Audio {
    sender: Option<Sender<Sound>>,
}

impl Audio {
    /// Создаёт проигрыватель. Звуковое устройство при этом не открывается.
    pub fn new() -> Self {
        let (sender, receiver) = channel::<Sound>();
        let worker = move || {
            let bank: HashMap<Sound, Vec<f32>> = Sound::ALL.iter().map(|&s| (s, synthesize(s))).collect();
            // Поток вывода должен жить, пока звучит звук, и не может покидать свой поток.
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
                    // Устройство пропало: при следующем звуке откроем его заново.
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

    /// Ставит звук в очередь воспроизведения.
    pub fn play(&self, sound: Sound) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(sound);
        }
    }
}
