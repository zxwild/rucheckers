//! Системные диалоги выбора файла.
//!
//! На macOS диалог должен открываться в главном потоке и сам ведёт цикл событий.
//! На остальных системах он открывается в отдельном потоке, чтобы окно продолжало
//! перерисовываться, пока пользователь выбирает файл.

use rucheckers::i18n::{tr, Lang, Text};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

/// Расширение файлов партий.
pub const EXTENSION: &str = "pdn";

/// Что делать с выбранным файлом.
pub enum FileAction {
    Open,
    /// Сохранить подготовленный текст PDN.
    Save(String),
}

/// Открытый диалог выбора файла.
pub struct FileJob {
    pub action: FileAction,
    receiver: Receiver<Option<PathBuf>>,
}

/// Состояние диалога.
pub enum FileJobState {
    /// Пользователь ещё выбирает.
    Pending,
    /// Диалог закрыт без выбора.
    Cancelled,
    /// Выбран файл.
    Chosen(PathBuf),
}

impl FileJob {
    /// Открывает диалог. `default_name` — предлагаемое имя файла при сохранении.
    pub fn start(action: FileAction, lang: Lang, default_name: &str) -> FileJob {
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
            // Если поток создать не удалось, канал закрывается и диалог считается отменённым.
            let _ = std::thread::Builder::new()
                .name("rucheckers-dialog".to_string())
                .spawn(run);
        }
        FileJob { action, receiver }
    }

    /// Проверяет, закрыт ли диалог.
    pub fn poll(&self) -> FileJobState {
        match self.receiver.try_recv() {
            Ok(Some(path)) => FileJobState::Chosen(path),
            Ok(None) | Err(TryRecvError::Disconnected) => FileJobState::Cancelled,
            Err(TryRecvError::Empty) => FileJobState::Pending,
        }
    }
}

/// Добавляет расширение `.pdn`, если пользователь его не указал.
pub fn with_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(EXTENSION);
    }
    path
}
