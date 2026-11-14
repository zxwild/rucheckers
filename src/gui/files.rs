//! System file dialogs.
//!
//! On macOS the dialog must be opened on the main thread and runs the event loop itself.
//! On other systems it is opened in a separate thread, so that the window keeps
//! repainting while the user is choosing a file.

use rucheckers::i18n::{tr, Lang, Text};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

/// Extension of game files.
pub const EXTENSION: &str = "pdn";

/// What to do with the chosen file.
pub enum FileAction {
    Open,
    /// Save the prepared PDN text.
    Save(String),
}

/// An open file dialog.
pub struct FileJob {
    pub action: FileAction,
    receiver: Receiver<Option<PathBuf>>,
}

/// State of the dialog.
pub enum FileJobState {
    /// The user is still choosing.
    Pending,
    /// The dialog was closed without a choice.
    Cancelled,
    /// A file was chosen.
    Chosen(PathBuf),
}

impl FileJob {
    /// Opens the dialog. `default_name` is the file name suggested for saving.
    pub fn start(action: FileAction, lang: Lang, default_name: &str) -> Self {
        let (sender, receiver) = channel();
        let saving = matches!(action, FileAction::Save(_));
        let title = tr(lang, if saving { Text::SaveTitle } else { Text::OpenTitle }).to_string();
        let filter = tr(lang, Text::PdnFilter).to_string();
        let name = default_name.to_string();
        let run = move || {
            let dialog = rfd::FileDialog::new()
                .set_title(title)
                .add_filter(filter, &[EXTENSION]);
            let chosen = if saving {
                dialog.set_file_name(name).save_file()
            } else {
                dialog.pick_file()
            };
            let _ = sender.send(chosen);
        };
        #[cfg(target_os = "macos")]
        run();
        #[cfg(not(target_os = "macos"))]
        {
            // If the thread cannot be created, the channel closes and the dialog counts as
            // cancelled.
            let _ = std::thread::Builder::new()
                .name("rucheckers-dialog".to_string())
                .spawn(run);
        }
        Self { action, receiver }
    }

    /// Checks whether the dialog has been closed.
    pub fn poll(&self) -> FileJobState {
        match self.receiver.try_recv() {
            Ok(Some(path)) => FileJobState::Chosen(path),
            Ok(None) | Err(TryRecvError::Disconnected) => FileJobState::Cancelled,
            Err(TryRecvError::Empty) => FileJobState::Pending,
        }
    }
}

/// Adds the `.pdn` extension if the user gave none.
pub fn with_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(EXTENSION);
    }
    path
}
