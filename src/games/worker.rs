//! Cancellable background search shared by game modes.

use super::{choose_move, GameState, Move, Position};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;

#[derive(Debug)]
pub enum JobStatus {
    Running,
    Done(Option<Move>),
    Failed,
}

#[derive(Debug)]
pub struct SearchJob {
    receiver: Receiver<Option<Move>>,
    stop: Arc<AtomicBool>,
}
impl SearchJob {
    #[must_use]
    pub fn start(state: GameState, history: Vec<Position>, level: u8, seed: u64) -> Self {
        let (sender, receiver) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let _ = thread::Builder::new()
            .name("board-game-ai".into())
            .spawn(move || {
                let answer = choose_move(state, &history, level, seed, &flag);
                let _ = sender.send(answer);
            });
        Self { receiver, stop }
    }
    #[must_use]
    pub fn poll(&self) -> JobStatus {
        match self.receiver.try_recv() {
            Ok(answer) => JobStatus::Done(answer),
            Err(TryRecvError::Empty) => JobStatus::Running,
            Err(TryRecvError::Disconnected) => JobStatus::Failed,
        }
    }
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for SearchJob {
    fn drop(&mut self) {
        self.cancel();
    }
}
