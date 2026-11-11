//! Расчёт хода в фоновом потоке: интерфейс не ждёт компьютер и может прервать его.

use super::{choose_move, Choice};
use crate::board::Position;
use crate::rules::GameState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;

/// Состояние фонового расчёта.
#[derive(Debug)]
pub enum JobStatus {
    /// Компьютер ещё думает.
    Running,
    /// Расчёт закончен. `None` — допустимых ходов нет.
    Done(Option<Choice>),
    /// Поток расчёта завершился, не прислав ответа.
    Failed,
}

/// Фоновый расчёт одного хода. При уничтожении расчёт прерывается.
#[derive(Debug)]
pub struct SearchJob {
    receiver: Receiver<Option<Choice>>,
    stop: Arc<AtomicBool>,
}

impl SearchJob {
    /// Запускает расчёт хода в отдельном потоке.
    pub fn start(state: GameState, history: Vec<Position>, level: u8, seed: u64) -> SearchJob {
        let (sender, receiver) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let work = move || {
            let choice = choose_move(&state, &history, level, seed, &flag);
            // Получателя может уже не быть: расчёт отменён.
            let _ = sender.send(choice);
        };
        // Если поток создать не удалось, канал закрывается, и расчёт считается сорвавшимся.
        let _ = thread::Builder::new()
            .name("rucheckers-ai".to_string())
            .spawn(work);
        SearchJob { receiver, stop }
    }

    /// Проверяет, готов ли ответ, не блокируя вызывающий поток.
    pub fn poll(&self) -> JobStatus {
        match self.receiver.try_recv() {
            Ok(choice) => JobStatus::Done(choice),
            Err(TryRecvError::Empty) => JobStatus::Running,
            Err(TryRecvError::Disconnected) => JobStatus::Failed,
        }
    }

    /// Просит расчёт остановиться как можно скорее.
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for SearchJob {
    fn drop(&mut self) {
        self.cancel();
    }
}
