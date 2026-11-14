//! Move calculation in a background thread: the interface does not wait for the computer
//! and can interrupt it.

use super::{choose_move, Choice};
use crate::board::Position;
use crate::rules::GameState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;

/// State of a background calculation.
#[derive(Debug)]
pub enum JobStatus {
    /// The computer is still thinking.
    Running,
    /// The calculation is finished. `None` means there are no legal moves.
    Done(Option<Choice>),
    /// The calculation thread ended without sending an answer.
    Failed,
}

/// Background calculation of one move. Dropping it interrupts the calculation.
#[derive(Debug)]
pub struct SearchJob {
    receiver: Receiver<Option<Choice>>,
    stop: Arc<AtomicBool>,
}

impl SearchJob {
    /// Starts calculating a move in a separate thread.
    #[must_use]
    pub fn start(state: GameState, history: Vec<Position>, level: u8, seed: u64) -> Self {
        let (sender, receiver) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let work = move || {
            let choice = choose_move(&state, &history, level, seed, &flag);
            // The receiver may be gone already: the calculation was cancelled.
            let _ = sender.send(choice);
        };
        // If the thread cannot be created, the channel closes and the calculation counts as failed.
        let _ = thread::Builder::new()
            .name("rucheckers-ai".to_string())
            .spawn(work);
        Self { receiver, stop }
    }

    /// Checks whether the answer is ready, without blocking the calling thread.
    #[must_use]
    pub fn poll(&self) -> JobStatus {
        match self.receiver.try_recv() {
            Ok(choice) => JobStatus::Done(choice),
            Err(TryRecvError::Empty) => JobStatus::Running,
            Err(TryRecvError::Disconnected) => JobStatus::Failed,
        }
    }

    /// Asks the calculation to stop as soon as possible.
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for SearchJob {
    fn drop(&mut self) {
        self.cancel();
    }
}
